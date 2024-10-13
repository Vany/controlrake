package httpserver

import (
	"context"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/httpserver/api"
	obsbrowser_api "github.com/vany/controlrake/src/obsbrowser/api"
	"github.com/vany/controlrake/src/widget"
	widget_api "github.com/vany/controlrake/src/widget/api"
	. "github.com/vany/pirog"
	"golang.org/x/net/websocket"
	"io"
	"net"
	"net/http"
	"strings"
)

const SERVER = "SERVER"

type HTTPServer struct {
	Cfg        *httpserver_api.Config
	Config     *config.Config            `inject:"Config"`
	Logger     *zerolog.Logger           `inject:"Logger"`
	ObsBrowser obsbrowser_api.ObsBrowser `inject:"ObsBrowser"`
	Widgets    widget_api.WidgetRegistry `inject:"Widgets"`

	Server             *http.Server
	HandlersToRegister map[string]http.Handler
}

func New() *HTTPServer {
	return &HTTPServer{
		HandlersToRegister: make(map[string]http.Handler),
	}
}

func (s *HTTPServer) Init(ctx context.Context) error {
	s.Cfg = &s.Config.HTTP
	ctx = context.WithValue(ctx, SERVER, s)
	s.Server = &http.Server{
		Addr:        s.Cfg.Addr,
		ConnState:   nil,
		BaseContext: func(net.Listener) context.Context { return ctx },
	}
	s.Logger = REF(s.Logger.With().Str("Comp", "HTTP").Logger())
	s.Logger.Info().Msg("Initialized")
	return nil
}

func (s *HTTPServer) Run(ctx context.Context) error {
	s.Server.Handler = s.Mux(ctx)
	go func() {
		if err := s.Server.ListenAndServe(); err == http.ErrServerClosed {
			s.Logger.Info().Msg("http server shut down gracefully")
		} else {
			panic("can't http server: " + err.Error())
		}
	}()
	s.Logger.Info().Msg("Launched")
	return nil
}

func (s *HTTPServer) Stop(ctx context.Context) error {
	s.Server.Shutdown(ctx)
	return nil
}

func (s *HTTPServer) RegisterHandler(path string, handler http.Handler) {
	s.HandlersToRegister[path] = handler
}

func (s *HTTPServer) GetBaseUrl(host string) string {
	ba := s.Server.Addr
	bindparts := strings.SplitN(ba, ":", 2)
	if len(bindparts) < 2 {
		bindparts[0] = ""
	} else {
		bindparts[0] = ":" + bindparts[1]
	}

	return "http://" + host + bindparts[0] + "/"
}

func (s *HTTPServer) Mux(ctx context.Context) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/static/", 302)
	})

	const sta = "/static/"
	mux.Handle(sta,
		http.StripPrefix(sta,
			http.FileServer(http.Dir(s.Cfg.StaticRoot)),
		))

	mux.Handle("/sound/",
		http.StripPrefix("/sound/",
			http.FileServer(http.Dir(s.Cfg.SoundRoot)),
		))

	mux.HandleFunc("/widgets/", RenderWidgets)

	///+	mux.Handle("/ws", websocket.Handler(CreateWsHandleFunc(ctx, s.Widgets)))
	mux.Handle("/wsobs", websocket.Handler(CreateWsHandleFunc(ctx, s.ObsBrowser)))

	for path, handler := range s.HandlersToRegister {
		mux.Handle(path, handler)
	}
	s.HandlersToRegister = nil

	return &LoggingHandler{mux}
}

type BaseWidget interface{ Base() *widget.BaseWidget }

// TODO - widgets paths nust be in widgets component
func RenderWidgets(w http.ResponseWriter, r *http.Request) {
	s := r.Context().Value(SERVER).(*HTTPServer)
	wi := s.Widgets
	path := strings.Split(r.URL.Path, "/")[2:]
	if root, ok := wi.(BaseWidget); !ok {
	} else if root.Base().Name != path[0] {
	} else {
		for _, p := range path[1:] {
			if ww, ok := wi.(*widget.Container); !ok {
				break
			} else if wi, ok = ww.Map[p]; !ok {
				wi = r.Context().Value(SERVER).(*HTTPServer).Widgets
				break
			}
		}
	}

	if err := wi.RenderTo(r.Context(), w); err != nil {
		s.Logger.Error().Err(err).Send()
	}
}

type LoggingHandler struct{ http.Handler }

func (h *LoggingHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s := r.Context().Value(SERVER).(*HTTPServer)
	h.Handler.ServeHTTP(w, r)
	s.Logger.Info().Str("url", r.URL.String()).Send()
}

// WsSubsystem - subsystem with chan for websocket
type WsSubsystem interface {
	Dispatch(ctx context.Context, b string) error // receive event from obs browser html
	SendChan() chan string                        // channel from server to page
}

func CreateWsHandleFunc(ctx context.Context, subsystem WsSubsystem) func(conn *websocket.Conn) {
	cpc := FANOUT(subsystem.SendChan())
	return func(ws *websocket.Conn) {
		defer ws.Close()
		ctx := ws.Request().Context()
		s := ctx.Value(SERVER).(*HTTPServer)
		log := s.Logger
		log.Debug().Str("url", ws.Request().URL.String()).Msg("I'm in ws handler")
		wsctx, cf := context.WithCancel(ctx)
		defer cf()

		send, senddetroy := cpc()
		defer senddetroy()

		go func() {
			for {
				select {
				case msg := <-send: // widgets.SendChan
					if fw, err := ws.NewFrameWriter(websocket.TextFrame); err != nil {
						log.Error().Err(err).Msg("Can't create new websocket frame")
					} else {
						fw.Write([]byte(msg))
					}
				case <-wsctx.Done():
					return
				}
			}
		}()

		for {
			f, err := ws.NewFrameReader()
			if err != nil {
				log.Error().Err(err).Msg("websocket failed")
				break
			}

			if f.PayloadType() == websocket.CloseFrame {
				log.Debug().Msg("Client wants to quit")
				break
			}

			log.Debug().Interface("payload", f.PayloadType()).Msg("New WS frame")

			b, err := io.ReadAll(f)

			if err != nil {
				log.Error().Err(err).Msg("websocket frame failed")
			} else {

				if err := subsystem.Dispatch(ctx, string(b)); err != nil {
					log.Error().Err(err).Msg("Receiver failed")
				} else {
					log.Info().Bytes("payload", b).Msg("websocket frame arrived")
				}
			}
		}
		log.Debug().Msg("websocket close")

	}
}

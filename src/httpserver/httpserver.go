package httpserver

import (
	"context"
	"errors"
	"io"
	"net"
	"net/http"
	"strings"

	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/httpserver/api"
	. "github.com/vany/pirog"
	"golang.org/x/net/websocket"
)

type HTTPServer struct {
	Cfg             *api.Config
	Config          *config.ConfigComponent `inject:"Config"`
	Logger          *zerolog.Logger         `inject:"Logger"`
	ObsBrowser      api.Comunicativo        `inject:"ObsBrowser"`
	WidgetComponent api.Widget              `inject:"WidgetComponent"`

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
	s.Logger = REF(s.Logger.With().Str("comp", "HTTP").Logger())
	s.Server = &http.Server{
		Addr:        s.Cfg.Addr,
		ConnState:   nil,
		BaseContext: func(net.Listener) context.Context { return ctx },
	}
	s.Logger.Info().Msg("Initialized")
	return nil
}

func (s *HTTPServer) Run(ctx context.Context) error {
	s.Server.Handler = &LoggingHandler{s.Mux(ctx), s.Logger}
	go func() {
		if err := s.Server.ListenAndServe(); errors.Is(err, http.ErrServerClosed) {
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
	mux.Handle("/", http.RedirectHandler("/static/index.html", 302))
	mux.Handle("/obs", http.RedirectHandler("/static/obs.html", 302))

	const fav = "/favicon.ico"
	mux.Handle(fav, http.RedirectHandler("/static"+fav, 302))

	const sta = "/static/"
	mux.Handle(sta, http.StripPrefix(sta, http.FileServer(http.Dir(s.Cfg.StaticRoot))))

	const sound = "/sound/"
	mux.Handle(sound, http.StripPrefix(sound, http.FileServer(http.Dir(s.Cfg.SoundRoot))))

	const widgets = "/widgets/"
	mux.Handle(widgets, http.StripPrefix(widgets, RenderWidgets{s}))

	mux.Handle("/ws", s.CreateWsHandleFunc(ctx, s.WidgetComponent))
	mux.Handle("/wsobs", s.CreateWsHandleFunc(ctx, s.ObsBrowser))

	for path, handler := range s.HandlersToRegister {
		mux.Handle(path, handler)
	}
	s.HandlersToRegister = nil

	return mux
}

type RenderWidgets struct{ *HTTPServer }

func (rw RenderWidgets) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if err := rw.WidgetComponent.RenderTo(r.Context(), r.URL.Path, w); err != nil {
		rw.Logger.Error().Err(err).Send()
	}
}

type LoggingHandler struct {
	H      http.Handler
	Logger *zerolog.Logger
}

func (h *LoggingHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	h.H.ServeHTTP(w, r)
	h.Logger.Info().Str("url", r.URL.String()).Send()
}

func (s *HTTPServer) CreateWsHandleFunc(ctx context.Context, subsystem api.Comunicativo) websocket.Handler {
	localChaner := FANOUT(subsystem.WebSpittoon())
	return func(ws *websocket.Conn) {
		defer ws.Close()
		log := s.Logger
		log.Debug().Str("url", ws.Request().URL.String()).Msg("I'm in ws handler")
		wsctx, cf := context.WithCancel(ctx)
		defer cf()

		send, senddetroy := localChaner()
		defer senddetroy()

		go func() {
			for {
				select {
				case msg := <-send: // widgets.Spittoon tee
					if fw, err := ws.NewFrameWriter(websocket.TextFrame); err != nil {
						log.Error().Err(err).Msg("Can't create new websocket frame")
					} else {
						fw.Write([]byte(msg))
					}
				case <-wsctx.Done():
					log.Debug().Msg("WS Service terminated")

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

			if b, err := io.ReadAll(f); err != nil {
				log.Error().Err(err).Msg("websocket frame failed")

			} else if err := subsystem.WebIngest(ctx, string(b)); err != nil {
				log.Error().Err(err).Msg("Receiver failed")

			} else {
				log.Debug().Msg("websocket frame served")
			}

		}
		log.Debug().Msg("websocket close")
	}
}

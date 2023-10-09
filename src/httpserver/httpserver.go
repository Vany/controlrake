package httpserver

import (
	"context"
	"fmt"
	"github.com/vany/controlrake/src/app"
	"golang.org/x/net/websocket"
	"io"
	"net"
	"net/http"
	"strings"
	"sync"
)

type HTTPServer struct {
	Server             *http.Server
	HandlersToRegister map[string]http.Handler
}

func New(ctx context.Context) (*HTTPServer, error) {
	s := new(HTTPServer)
	con := app.FromContext(ctx)
	s.HandlersToRegister = make(map[string]http.Handler)
	s.Server = &http.Server{
		Addr:        con.Cfg.BindAddress,
		ConnState:   nil,
		BaseContext: func(net.Listener) context.Context { return ctx },
	}

	go func() {
		select {
		case <-ctx.Done():
			s.Server.Shutdown(ctx)
		}
	}()

	return s, nil
}

func (s *HTTPServer) RegisterHandler(path string, handler http.Handler) {
	s.HandlersToRegister[path] = handler
}

func (s *HTTPServer) InitStage1(ctx context.Context) error {
	s.Server.Handler = s.Mux(ctx)
	go func() {
		if err := s.Server.ListenAndServe(); err == http.ErrServerClosed {
			app.FromContext(ctx).Log.Info().Msg("http server shut down gracefully")
		} else {
			panic("can't http server: " + err.Error())
		}
	}()
	return nil
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
	app := app.FromContext(ctx)
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/static/", 302)
	})

	const sta = "/static/"
	mux.Handle(sta,
		http.StripPrefix(sta,
			http.FileServer(http.Dir(app.Cfg.StaticRoot)),
		))

	mux.Handle("/sound/",
		http.StripPrefix("/sound/",
			http.FileServer(http.Dir(app.Cfg.SoundRoot)),
		))

	mux.HandleFunc("/googleoauth2", GoogleOAUTH2(ctx))

	mux.HandleFunc("/widgets/", RenderWidgets)
	mux.Handle("/ws", websocket.Handler(CreateWsHandleFunc(ctx, app.Widget)))
	mux.Handle("/wsobs", websocket.Handler(CreateWsHandleFunc(ctx, app.ObsBrowser)))

	for path, handler := range s.HandlersToRegister {
		mux.Handle(path, handler)
	}
	s.HandlersToRegister = nil

	return &LoggingHandler{mux}
}

// TODO put this to youtube package
func GoogleOAUTH2(ctx context.Context) func(http.ResponseWriter, *http.Request) {
	return func(w http.ResponseWriter, r *http.Request) {
		app := app.FromContext(ctx)
		code := r.FormValue("code")
		w.Header().Set("Content-Type", "text/plain")
		fmt.Fprintf(w, "Received code: %v\r\nYou can now safely close this browser window.", code)
		if cch := app.Youtube.GetCodeChan(); cch == nil {
			app.Log.Error().Msg("Codechan is nil")
		} else {
			cch <- code
		}
	}
}

func RenderWidgets(w http.ResponseWriter, r *http.Request) {
	con := app.FromContext(r.Context())
	if err := con.Widget.RenderTo(r.Context(), w); err != nil {
		con.Log.Error().Err(err).Send()
	}
}

type LoggingHandler struct{ http.Handler }

func (h *LoggingHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := app.FromContext(r.Context())
	h.Handler.ServeHTTP(w, r)
	con.Log.Info().Str("url", r.URL.String()).Send()
}

// WsSubsystem - subsystem with chan for websocket
type WsSubsystem interface {
	Dispatch(ctx context.Context, b []byte) error // receive event from obs browser html
	SendChan() chan string                        // channel from server to page
}

func CreateWsHandleFunc(ctx context.Context, subsystem WsSubsystem) func(conn *websocket.Conn) {
	cpc := COPYCHAN(ctx.Done(), subsystem.SendChan())
	return func(ws *websocket.Conn) {
		defer ws.Close()
		ctx := ws.Request().Context()
		con := app.FromContext(ctx)
		con.Log.Debug().Str("url", ws.Request().URL.String()).Msg("I'm in ws handler")

		wsctx, cf := context.WithCancel(ctx)
		defer cf()

		send, senddetroy := cpc()
		defer senddetroy()

		go func() {
			for {
				select {
				case msg := <-send: // widgets.SendChan
					if fw, err := ws.NewFrameWriter(websocket.TextFrame); err != nil {
						con.Log.Error().Err(err).Msg("Can't create new websocket frame")
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
				con.Log.Error().Err(err).Msg("websocket failed")
				break
			}

			if f.PayloadType() == websocket.CloseFrame {
				con.Log.Debug().Msg("Client wants to quit")
				break
			}

			con.Log.Debug().Interface("payload", f.PayloadType()).Msg("New WS frame")

			b, err := io.ReadAll(f)

			if err != nil {
				con.Log.Error().Err(err).Msg("websocket frame failed")
			} else {

				if err := subsystem.Dispatch(ctx, b); err != nil {
					con.Log.Error().Err(err).Msg("Receiver failed")
				} else {
					con.Log.Info().Bytes("payload", b).Msg("websocket frame arrived")
				}
			}
		}
		con.Log.Debug().Msg("websocket close")

	}
}

func COPYCHAN[T any](done <-chan struct{}, src chan T) (
	generator func() (tap chan T, destructor func()),
) {
	var mu sync.RWMutex
	chans := make(map[chan T]struct{})

	go func() {
		for {
			select {
			case msg := <-src:
				mu.RLock()
				for c, _ := range chans {
					c <- msg
				}
				mu.RUnlock()
			case <-done:
				mu.Lock()
				for c, _ := range chans {
					close(c)
					delete(chans, c)
				}
				mu.Unlock()
				return
			}
		}
	}()

	return func() (tap chan T, destructor func()) {
		ret := make(chan T)
		mu.Lock()
		chans[ret] = struct{}{}
		mu.Unlock()
		return ret, func() {
			mu.Lock()
			delete(chans, ret)
			mu.Unlock()
		}
	}
}

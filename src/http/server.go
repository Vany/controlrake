package http

import (
	"context"
	"github.com/vany/controlrake/src/app"
	"golang.org/x/net/websocket"
	"io"
	"net"
	"net/http"
)

func ListenAndServe(ctx context.Context) error {
	con := app.FromContext(ctx)
	httpServer := http.Server{
		Addr:        con.Cfg.BindAddress,
		Handler:     Mux(ctx),
		ConnState:   nil,
		BaseContext: func(net.Listener) context.Context { return ctx },
	}

	go func() {
		select {
		case <-ctx.Done():
			httpServer.Shutdown(ctx)
		}
	}()

	if err := httpServer.ListenAndServe(); err == http.ErrServerClosed {
		app.FromContext(ctx).Log.Info().Msg("http server shut down gracefully")
	} else {
		return err
	}

	return nil
}

func Mux(ctx context.Context) http.Handler {
	con := app.FromContext(ctx)
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/static/", 302)
	})

	const sta = "/static/"
	mux.Handle(sta,
		http.StripPrefix(sta,
			http.FileServer(http.Dir(con.Cfg.StaticRoot)),
		))

	mux.Handle("/sound/",
		http.StripPrefix("/sound/",
			http.FileServer(http.Dir(con.Cfg.SoundRoot)),
		))

	mux.Handle("/widgets/", &WidgetList{})

	widgets := con.Widgets
	mux.Handle("/ws", websocket.Handler(CreateWsHandleFunc(widgets.SendChan(), widgets.Dispatch)))

	ow := con.ObsBrowser
	mux.Handle("/wsobs", websocket.Handler(CreateWsHandleFunc(ow.SendChan(), ow.Dispatch)))

	return &LoggingHandler{mux}
}

type LoggingHandler struct{ http.Handler }

func (h *LoggingHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := app.FromContext(r.Context())
	h.Handler.ServeHTTP(w, r)
	con.Log.Info().Str("url", r.URL.String()).Send()
}

type WidgetList struct {
	http.Header
}

func (h *WidgetList) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := app.FromContext(r.Context())
	if err := con.Widgets.RenderTo(r.Context(), w); err != nil {
		con.Log.Error().Err(err).Send()
	}
}

func CreateWsHandleFunc(send chan string, receive func(context.Context, []byte) error) func(conn *websocket.Conn) {
	return func(ws *websocket.Conn) {
		defer ws.Close()
		ctx := ws.Request().Context()
		con := app.FromContext(ctx)
		con.Log.Debug().Str("url", ws.Request().URL.String()).Msg("I'm in ws handler")

		wsctx, cf := context.WithCancel(ctx)
		defer cf()

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

				if err := receive(ctx, b); err != nil {
					con.Log.Error().Err(err).Msg("Receiver failed")
				} else {
					con.Log.Info().Bytes("payload", b).Msg("websocket frame arrived")
				}
			}
		}
		con.Log.Debug().Msg("websocket close")

	}

}

package http

import (
	"context"
	"github.com/vany/controlrake/src/cont"
	"golang.org/x/net/websocket"
	"io"
	"net"
	"net/http"
)

func ListenAndServe(ctx context.Context) error {
	con := cont.FromContext(ctx)
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
		cont.FromContext(ctx).Log.Info().Msg("http server shut down gracefully")
	} else {
		return err
	}

	return nil
}

func Mux(ctx context.Context) http.Handler {
	con := cont.FromContext(ctx)
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/static/", 302)
	})

	const sta = "/static/"
	mux.Handle(sta,
		http.StripPrefix(sta,
			http.FileServer(http.Dir(con.Cfg.StaticRoot)),
		))

	mux.Handle("/widgets/", &WidgetList{})
	mux.Handle("/ws", websocket.Handler(WShandle))

	return &LoggingHandler{mux}
}

type LoggingHandler struct{ http.Handler }

func (h *LoggingHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := cont.FromContext(r.Context())
	h.Handler.ServeHTTP(w, r)
	con.Log.Info().Str("url", r.URL.String()).Send()
}

type WidgetList struct {
	http.Header
}

func (h *WidgetList) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := cont.FromContext(r.Context())
	if err := con.Widgets.RenderTo(r.Context(), w); err != nil {
		con.Log.Error().Err(err).Send()
	}
}

// TODO keep ws registry ands send from SendChan to every of it

func WShandle(ws *websocket.Conn) {
	defer ws.Close()
	ctx := ws.Request().Context()
	con := cont.FromContext(ctx)
	con.Log.Debug().Msg("I'm in ws handler")
	widgets := con.Widgets

	wsctx, cf := context.WithCancel(ctx)
	defer cf()
	go func() {
		for {
			select {
			case msg := <-widgets.SendChan():
				ws.Write([]byte(msg))
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

		con.Log.Debug().Interface("payload", f.PayloadType()).Msg("New WS frame")
		b, err := io.ReadAll(f)
		if err != nil {
			con.Log.Error().Err(err).Msg("websocket frame failed")
		} else {
			widgets.Consume(ctx, b)
			con.Log.Info().Bytes("payload", b).Msg("websocket frame arrived")
		}
	}
	con.Log.Debug().Msg("websocket close")

}

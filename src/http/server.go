package http

import (
	"bytes"
	"context"
	"github.com/vany/controlrake/src/types"
	"github.com/vany/controlrake/src/widget"
	. "github.com/vany/pirog"
	"net"
	"net/http"
)

type Server struct {
	Events chan<- any
}

func ListenAndServe(ctx context.Context) error {
	con := types.FromContext(ctx)
	httpServer := http.Server{
		Addr:        con.Cfg.BindAddress,
		Handler:     Mux(ctx),
		ConnState:   nil,
		BaseContext: func(net.Listener) context.Context { return ctx },
	}
	return httpServer.ListenAndServe()
}

func Mux(ctx context.Context) http.Handler {
	con := types.FromContext(ctx)
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

	return &LoggingHandler{mux}
}

type LoggingHandler struct{ http.Handler }

func (h *LoggingHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := types.FromContext(r.Context())
	h.Handler.ServeHTTP(w, r)
	con.Log.Info().Str("url", r.URL.String()).Send()

}

type WidgetList struct {
	http.Header
}

func (h *WidgetList) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	con := types.FromContext(r.Context())
	ret := MAP(VALUES(con.Widgets), func(win widget.Widget) []byte {
		b := bytes.Buffer{}
		win.RenderTo(&b)
		return b.Bytes()
	})
	if _, err := w.Write(bytes.Join(ret, []byte{'\n'})); err != nil {
		con.Log.Error().Err(err).Send()
	}
}

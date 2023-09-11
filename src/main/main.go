// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2020

package main

import (
	"context"
	"github.com/vany/controlrake/src/types"
	"net/http"
)

func main() {
	ctx := types.ReadConfigToContext(context.Background())

	// components.serve()

	cfg, _ := types.FromContext(ctx)
	http.ListenAndServe(cfg.BindAddress, Mux(ctx))
}

func Mux(ctx context.Context) http.Handler {
	cfg, _ := types.FromContext(ctx)
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/static", 302)
	})

	mux.Handle("/static", http.FileServer(http.Dir(cfg.StaticRoot)))

	return mux
}

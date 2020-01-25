package webserver

import (
	"context"
	"github.com/Vany/controlrake/src/config"
	"log"
	"net/http"
	"os"
)

type webServer struct {
	cfg config.CfgType
}

func New(cfg config.CfgType) *webServer {
	w := new(webServer)
	w.cfg = cfg
	return w
}

func (w *webServer) Run(ctx context.Context) *webServer {
	go func() {
		mux := http.NewServeMux()
		lFunc1 := logging(log.New(os.Stdout, "[1]", 0))
		lFunc2 := logging(log.New(os.Stdout, "[2]", 0))
		mux.Handle("/public/", lFunc1(http.FileServer(http.Dir("."))))
		mux.Handle("/", lFunc2(http.FileServer(http.Dir("./public/"))))
		mux.Handle("/ws", new(WebSocket))
		http.ListenAndServe(":8080", mux)
	}()
	return w
}

func (w *webServer) Stop() *webServer {
	return w
}

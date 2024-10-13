package httpserver_api

import "net/http"

type Config struct {
	Addr       string
	StaticRoot string
	SoundRoot  string
}

type HTTPServer interface {
	GetBaseUrl(host string) string // get base url for serving on host
	RegisterHandler(path string, handler http.Handler)
}

var Instance HTTPServer

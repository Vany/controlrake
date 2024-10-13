package api

import (
	"golang.org/x/net/context"
	"io"
	"net/http"
)

type Config struct {
	Addr       string
	StaticRoot string
	SoundRoot  string
}

type HTTPServer interface {
	GetBaseUrl(host string) string // get base url for serving on host
	RegisterHandler(path string, handler http.Handler)
}

// Comunicativo - can communicate with websocket
type Comunicativo interface {
	WebIngest(ctx context.Context, data string) error
	WebSpittoon() chan string
}

type Widget interface {
	Comunicativo
	RenderTo(ctx context.Context, arg string, w io.Writer) error
}

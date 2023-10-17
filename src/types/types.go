package types

import (
	"context"
	"github.com/andreykaipov/goobs"
	"io"
	"net/http"
)

type Component interface {
	Ready() bool
}

type WidgetRegistry interface {
	Dispatch(ctx context.Context, b string) error
	SendChan() chan string
	RenderTo(ctx context.Context, w io.Writer) error
}

type Obs interface {
	Cli() *goobs.Client // get raw client
}

type ObsBrowser interface {
	Send(ctx context.Context, msg string) ObsSendObject // send message to obs browser html
	Dispatch(ctx context.Context, b string) error       // receive event from obs browser html
	SendChan() chan string                              // channel from server to page
}

type ObsSendObject interface {
	Done() chan struct{}  // will be closed when action was finished
	Receive() chan string // return action progress messages channel
}

type Youtube interface {
	Component
	GetCodeChan() chan string // get channel to return code from oauth
}

type HTTPServer interface {
	GetBaseUrl(host string) string // get base url for serving on host
	RegisterHandler(path string, handler http.Handler)
}

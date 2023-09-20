package types

import (
	"context"
	"encoding/json"
	"github.com/andreykaipov/goobs"
	"io"
)

type Connector interface {
	Handle(method string, arg interface{})
	Init() Connector
}

type WebMessage struct {
	Module string          `json:"module"`
	Method string          `json:"method"`
	Arg    json.RawMessage `json:"arg"`
}

type WidgetRegistry interface {
	Dispatch(ctx context.Context, b []byte) error
	SendChan() chan string
	RenderTo(ctx context.Context, w io.Writer) error
}

type Obs interface {
	Cli() *goobs.Client // get raw client
}

type ObsBrowser interface {
	Send(ctx context.Context, msg string) ObsSendObject // send message to obs browser html
	Dispatch(ctx context.Context, b []byte) error       // receive event from obs browser html
	SendChan() chan string                              // channel from server to page
}

type ObsSendObject interface {
	Done() chan struct{}  // will be closed when action was finished
	Receive() chan string // return action progress messages channel
}

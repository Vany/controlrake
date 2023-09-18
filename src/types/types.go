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
	Consume(ctx context.Context, b []byte)
	SendChan() chan string
	RenderTo(ctx context.Context, w io.Writer) error
}

type Sound interface {
	Play(ctx context.Context, fname string) error
}

type Obs interface {
	Cli() *goobs.Client // get raw client
}

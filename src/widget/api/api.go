package api

import (
	"context"
	"io"
)

type Config struct {
	Name    string // Unique widget id
	Type    string // Type of widget class
	Caption string // Text to render in widget if it is a button or something like this
	Style   string // css style for this widget only
	Args    any    // Widget specific config
}

type WidgetRegistry interface {
	Dispatch(ctx context.Context, b string) error
	SendChan() chan string
	RenderTo(ctx context.Context, w io.Writer) error
}

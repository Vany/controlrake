package api

import (
	"context"
	"io"
)

type Config struct {
	Root WidgetConfig
}

type WidgetConfig struct {
	Name    string // Unique widget id
	Type    string // Type of widget class
	Caption string // Text to render in widget if it is a button or something like this
	Style   string // css style for this widget only
	Args    any    // Widget specific config
}

type WidgetComponent interface{}

type Widget interface {
	Init(ctx context.Context, c WidgetConstructor) error          // init widget with config in it's base
	Dispatch(ctx context.Context, event string) error             // consume one event from Websocket
	RenderTo(ctx context.Context, arg string, wr io.Writer) error // render visual representation
	RenderFuncs() map[string]any                                  // Additional functions for template
	Errorf(f string, args ...any) error
}

type WidgetConstructor interface {
	NewWidget(ctx context.Context, cfg *WidgetConfig) (Widget, error)
	GetComponent(name string) any
}

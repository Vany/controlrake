package types

import (
	"context"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/widget"
)

var Key = struct{}{}

type Context struct {
	Cfg     *config.Config
	Log     *Logger
	Widgets widget.Registry
}

func PutToContext(ctx context.Context, obj any) context.Context {
	c := FromContext(ctx)
	if c == nil {
		c = &Context{}
		ctx = context.WithValue(ctx, Key, c)
	}
	switch to := obj.(type) {
	case *config.Config:
		c.Cfg = to
	case *Logger:
		c.Log = to
	case widget.Registry:
		c.Widgets = to
	default:
		panic("unknown type in context container")
	}
	return ctx
}

func FromContext(ctx context.Context) *Context {
	c := ctx.Value(Key) // or die
	if c == nil {
		return nil
	} else {
		return c.(*Context)
	}
}

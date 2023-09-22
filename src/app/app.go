package app

import (
	"context"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/types"
)

var Key = struct{}{}

// container with all my goodies
type App struct {
	Cfg        *config.Config
	Log        *types.Logger
	Widget     types.WidgetRegistry
	Obs        types.Obs
	ObsBrowser types.ObsBrowser
}

func PutToApp(ctx context.Context, obj any) context.Context {
	c := FromContext(ctx)
	if c == nil {
		c = &App{}
		ctx = context.WithValue(ctx, Key, c)
	}
	switch to := obj.(type) {
	case *config.Config:
		c.Cfg = to
	case *types.Logger:
		c.Log = to
	case types.WidgetRegistry:
		c.Widget = to
	case types.Obs:
		c.Obs = to
	case types.ObsBrowser:
		c.ObsBrowser = to
	default:
		panic("unknown type in context container")
	}
	return ctx
}

func FromContext(ctx context.Context) *App {
	c := ctx.Value(Key) // or die
	if c == nil {
		return nil
	} else {
		return c.(*App)
	}
}

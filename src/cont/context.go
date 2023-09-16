package cont

import (
	"context"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/types"
)

var Key = struct{}{}

// container with all my goodies
type Goodies struct {
	Cfg     *config.Config
	Log     *types.Logger
	Widgets types.WidgetRegistry
	Sound   types.Sound
	Obs     types.Obs
}

func PutToContext(ctx context.Context, obj any) context.Context {
	c := FromContext(ctx)
	if c == nil {
		c = &Goodies{}
		ctx = context.WithValue(ctx, Key, c)
	}
	switch to := obj.(type) {
	case *config.Config:
		c.Cfg = to
	case *types.Logger:
		c.Log = to
	case types.WidgetRegistry:
		c.Widgets = to
	case types.Sound:
		c.Sound = to
	case types.Obs:
		c.Obs = to
	default:
		panic("unknown type in context container")
	}
	return ctx
}

func FromContext(ctx context.Context) *Goodies {
	c := ctx.Value(Key) // or die
	if c == nil {
		return nil
	} else {
		return c.(*Goodies)
	}
}

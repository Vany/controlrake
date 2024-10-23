package widget

import (
	"context"
	"fmt"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/widget/api"
	"github.com/vany/controlrake/src/widget/impl"
	_ "github.com/vany/controlrake/src/widget/impl"
	"github.com/vany/pirog"
	"io"
	"reflect"
)

// Component - part of application initialization system
type Component struct {
	Config config.ConfigComponent `inject:"Config"`
	Logger *zerolog.Logger        `inject:"Logger"`

	Cfg  *api.Config
	Root api.Widget
}

func NewComponent() *Component {
	return &Component{}
}

func (c *Component) Init(ctx context.Context) (err error) {
	c.Cfg = pirog.REF(c.Config.Widget)
	c.Logger = pirog.REF(c.Logger.With().Str("comp", "widget").Logger())
	if c.Root, err = c.NewWidget(ctx, &c.Cfg.Root); err != nil {
		return err
	}
	return nil
}

func (c *Component) Run(ctx context.Context) error {
	return nil
}

func (c *Component) Stop(ctx context.Context) error {
	return nil
}

type Baser interface{ Base() *impl.BaseWidget }

func (c *Component) NewWidget(ctx context.Context, cfg *api.WidgetConfig) (api.Widget, error) {
	if t, ok := impl.TypeRegistry[cfg.Type]; !ok {
		return nil, fmt.Errorf("unknown widget type: %s", cfg.Type)
	} else {
		w := reflect.New(t).Interface().(api.Widget)
		// TODO split Base.Init and w.Init() w.Init(ctx, c, *cfg, Log, chan, chan, chan)
		w.(Baser).Base().WidgetConfig = *cfg
		w.(Baser).Base().Widget = w
		w.(Baser).Base().Log = c.Logger.With().Str("widget", cfg.Name).Logger()
		w.Init(ctx, c)
		return w, nil
	}
}

func (c *Component) Dispatch(buff []byte) error {
	return nil
}

// TODO do not RenderTo() use Render command  !!!! IMPORTANT

// @nonblocking
func (c *Component) RenderTo(ctx context.Context, arg string, w io.Writer) error {
	return c.Root.RenderTo(ctx, arg, w)
}

///////

// TODO - decide what to do with multiple obsbrowser pages
// 📍 receives messages, Send messages to web
// 📍 RenderTo is nonblocking

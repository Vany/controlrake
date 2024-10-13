package widget

import (
	"context"
	"github.com/vany/controlrake/src/config"
	widget_api "github.com/vany/controlrake/src/widget/api"
	"github.com/vany/pirog"
)

type Component struct {
	Widget
	Config config.Config `inject:"Config"`
	Cfg    *widget_api.Config
}

func NewComponent() *Component {
	return &Component{}
}

func (w *Component) Init(ctx context.Context) error {
	w.Cfg = pirog.REF(w.Config.Widget)
	return nil
}

func (w *Component) Run(ctx context.Context) error {
	return nil
}

func (w *Component) Stop(ctx context.Context) error {
	return nil
}

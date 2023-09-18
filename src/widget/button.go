package widget

import (
	"context"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/cont"
	. "github.com/vany/pirog"
)

type ButtonArgs struct {
	Action string
	Sound  string
}

type Button struct {
	BaseWidget
	Args ButtonArgs
}

var _ = MustSurvive(RegisterWidgetType(&Button{}, `
<button onClick="Send(this, 'Boo')">{{.Caption}}</button>
`))

func (w *Button) Init(context.Context) error {
	err := mapstructure.Decode(w.Config.Args, &w.Args)
	return TERNARY(err == nil, nil, w.Errorf("cant read config %#v: %w", w.Config.Args, err))
}

func (w *Button) Consume(ctx context.Context, event []byte) error {
	con := cont.FromContext(ctx)
	con.Log.Log().Bytes("event", event).Msg("Pressed")

	if w.Args.Sound != "" {
		cont.FromContext(ctx).Sound.Play(ctx, w.Args.Sound)
	}

	return nil
}

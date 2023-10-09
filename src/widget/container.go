package widget

import (
	"bytes"
	"context"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/pirog"
	"io"
)

type Container struct {
	BaseWidget

	// registry part
	Map   map[string]Widget // contained widgets
	Order []string          // order which widgets was in config
}

var _ = MustSurvive(RegisterWidgetType(&Container{}, ``))

func (w *Container) Init(ctx context.Context) error {
	w.Map = make(map[string]Widget)
	for _, cfga := range w.Args.([]any) {
		wnew := New(ctx, cfga)
		name := wnew.Base().Config.Name
		w.Map[name] = wnew
		wnew.Base().Chan = w.Chan
		w.Order = append(w.Order, name)
	}
	return nil
}

// TODO make it template based
func (w *Container) RenderTo(ctx context.Context, wr io.Writer) error {
	app := app.FromContext(ctx)
	// TODO make this rendering template based
	if _, err := wr.Write([]byte(`<div class="widget container" id="` + w.Name + `"` +
		pirog.TERNARY(w.Style != "", ` style="`+w.Style+`"`, "") +
		`>`)); err != nil {
		return err
	}
	for _, n := range w.Order {
		w := w.Map[n]
		if err := w.RenderTo(ctx, wr); err != nil {
			app.Log.Error().Err(err).Msgf("%s render failed", n)
			return w.Base().Errorf("%s render failed", n)
		}
	}
	if _, err := wr.Write([]byte(`</div>`)); err != nil {
		return err
	}

	return nil
}

func (w *Container) Dispatch(ctx context.Context, b []byte) error {
	parts := bytes.SplitN(b, []byte{'|'}, 2)
	name := string(parts[0])

	if win, ok := w.Map[name]; !ok {
		return w.Base().Errorf("widget %s not found", name)
	} else if err := win.Dispatch(ctx, parts[1]); err != nil {
		return win.Base().Errorf("can't dispatch '%s': %w", parts[1], err)
	}
	return nil
}

func (w *Container) Children() map[string]Widget {
	return w.Map
}

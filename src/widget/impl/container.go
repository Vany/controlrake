package impl

import (
	"bytes"
	"context"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/widget/api"
	"html/template"
	"io"
	"strings"
)

// Container - Widget that contains another, also passes events in and out obeying path rules
type Container struct {
	BaseWidget

	Map   map[string]api.Widget // contained widgets
	Order []string              // order which widgets was in config
	Len   int                   // count of widgets

	// volatile for rendering only
	CTX context.Context
	ARG string
}

var _ = RegisterWidgetType(&Container{}, `
<div class="container">{{ .RenderChildren .CTX .ARG }}</div>

<script>
	let self = document.getElementById("{{.Name}}")

	self.oncontextmenu = function (ev) {
		ev.stopImmediatePropagation();
		FetchWidgets(EvaluateMyPath(self).replaceAll("|", "/"));
	}
</script>
`)

func (w *Container) Init(ctx context.Context, c api.WidgetConstructor) error {
	if w.Args == nil {
		w.Log.Error().Msg("Empty Container")
		return nil
	}

	w.Map = make(map[string]api.Widget)
	for _, icfg := range w.Args.([]any) {

		cfg := new(api.WidgetConfig)
		if err := mapstructure.Decode(icfg, &cfg); err != nil {
			return w.Errorf("wrong widget config %v: %w", icfg, err)

		} else if wnew, err := c.NewWidget(ctx, cfg); err != nil {
			return w.Errorf("can't create widget %s: %w", cfg.Name, err)

		} else {
			// TODO put connectivity here if here is any
			w.Map[cfg.Name] = wnew
			w.Order = append(w.Order, cfg.Name)
		}
	}
	w.Len = len(w.Map)
	return nil
}

type Baser interface{ Base() *BaseWidget }

func (w *Container) Dispatch(ctx context.Context, b string) error {
	parts := strings.SplitN(b, "|", 2)
	if parts[0] == w.Name {
		parts = strings.SplitN(parts[1], "|", 2)
	}

	name := parts[0]
	if win, ok := w.Map[name]; !ok {
		return w.Base().Errorf("widget %s not found", name)
	} else if err := win.Dispatch(ctx, parts[1]); err != nil {
		return win.(Baser).Base().Errorf("can't dispatch '%s': %w", parts[1], err)
	}
	return nil
}

func (w *Container) RenderChildren(ctx context.Context, arg string) template.HTML {
	buff := bytes.NewBuffer(make([]byte, 0, 8192))
	for _, name := range w.Order {
		if err := w.Map[name].RenderTo(ctx, arg, buff); err != nil {
			w.Log.Error().Err(err).Msg("render error")
			buff.WriteString(w.Errorf("<div>%w</div>", err).Error())
		}
	}
	return template.HTML(buff.String())
}

func (w *Container) RenderTo(ctx context.Context, arg string, wr io.Writer) error {
	if arg == "" {
		w.CTX, w.ARG = ctx, arg
		return w.BaseWidget.RenderTo(ctx, arg, wr)
	}
	slash := strings.Index(arg, "/")
	name := arg
	path := ""
	if slash != -1 {
		name = arg[:slash]
		path = arg[slash+1:]
	}
	if widget, ok := w.Map[name]; !ok {
		return w.Errorf("can't find widget: %s", name)
	} else {
		return widget.RenderTo(ctx, path, wr)
	}
}

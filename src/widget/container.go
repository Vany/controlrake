package widget

import (
	"context"
	"strings"
)

type Container struct {
	BaseWidget

	// registry part
	Map   map[string]Widget // contained widgets
	Order []string          // order which widgets was in config
	Len   int               // count of widgets
}

var _ = MustSurvive(RegisterWidgetType(&Container{}, `
<div class="container" id="{{.Name}}" {{if .Style}}style="{{.Style}}{{end}}">
{{ $M := .Map }}
{{range .Order}}
{{ $v := index $M .}}
{{$buff := WriteBuffer -}} {{- $err := $v.RenderTo nil $buff -}} {{$buff.String}}
{{end}}
</div>

<script>
	let self = document.getElementById("{{.Name}}")

	self.oncontextmenu = function (ev) {
		ev.stopImmediatePropagation();
		FetchWidgets(EvaluateMyPath(self).replaceAll("|", "/"));
	}
</script>
`))

func (w *Container) Init(ctx context.Context) error {
	if w.Args == nil {
		w.Log.Error().Msg("Empty Container")
		return nil
	}
	w.Map = make(map[string]Widget)
	for _, cfga := range w.Args.([]any) {
		wnew := New(ctx, cfga)
		name := wnew.Base().Config.Name
		w.Map[name] = wnew
		wnew.Base().Chan = w.Chan
		w.Order = append(w.Order, name)
	}
	w.Len = len(w.Map)
	return nil
}

func (w *Container) Dispatch(ctx context.Context, b string) error {
	parts := strings.SplitN(b, "|", 2)
	if parts[0] == w.Name {
		parts = strings.SplitN(parts[1], "|", 2)
	}

	name := parts[0]
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

//<table class="container" id="{{.Name}}" width="100%"><tr>
//{{range $k, $v := .Map}}
//<td>{{$buff := WriteBuffer -}} {{- $err := $v.RenderTo nil $buff -}} {{$buff.String}}</td>
//{{end}}
//</tr></table>

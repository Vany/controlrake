package widget

import (
	"context"
	"html/template"
	"io"
)

type Label struct {
	BaseWidget
	Text string //🔴actually we do not need it, but for education purposes let's use it
}

var _ = MustSurvive(RegisterWidgetType(&Label{}))

func (w *Label) Init(context.Context) error {
	if s, ok := w.Args.(string); !ok {
		return w.Errorf("args is not string, but %#v", w.Args)
	} else {
		w.Text = s
	}
	return nil
}

func (w *Label) RenderTo(wr io.Writer) error {
	if err := TLabel.Execute(wr, w); err != nil {
		return w.Errorf("render failed: %w", err)
	}
	return nil
}

var TLabel = template.Must(template.New("Label").Parse(`
<div class="widget" id="{{.Name}}">
	<b>{{.Text}}</b>	
</div>
`))

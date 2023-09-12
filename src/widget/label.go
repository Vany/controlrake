package widget

import (
	"html/template"
	"io"
)

type Label struct {
	BaseWidget
	Text string //🔴actually we do not need it, but for education purposes let's use it
}

func (w *Label) Init() error {
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
<div style="border-width: thick ;border: black" id="{{.Name}}">
	<b>{{.Text}}</b>	
</div>
`))

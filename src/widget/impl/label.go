package impl

type Label struct {
	BaseWidget
}

var _ = RegisterWidgetType(&Label{}, `
	<b>{{.Caption}}</b>
`)

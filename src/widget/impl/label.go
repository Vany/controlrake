package impl

// Label - was for render development
type Label struct {
	BaseWidget
}

var _ = RegisterWidgetType(&Label{}, `
	<b>{{.Caption}}</b>
`)

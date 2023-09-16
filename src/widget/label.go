package widget

type Label struct {
	BaseWidget
}

var _ = MustSurvive(RegisterWidgetType(&Label{}, `
	<b>{{.Args}}</b>
`))

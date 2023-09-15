package widget

type Label struct {
	BaseWidget
}

var _ = MustSurvive(RegisterWidgetType(&Label{}, `
<div class="widget" id="{{.Name}}">
	<b>{{.Args}}</b>	
</div>
`))

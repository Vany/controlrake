package widget

type HrBr struct {
	BaseWidget
}

// TODO make it real choose between hr and br on config base
var _ = MustSurvive(RegisterWidgetType(&HrBr{}, `
	<br style="{{ .Args }}">
	<script>
		document.getElementById("{{.Name}}").style.border="none";
	</script>
`))

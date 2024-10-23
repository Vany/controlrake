package oldwidgets

import (
	"github.com/vany/controlrake/src/widget/impl"
)

type HrBr struct {
	impl.BaseWidget
}

// TODO make it real choose between hr and br on config base
var _ = MustSurvive(impl.RegisterWidgetType(&HrBr{}, `
	<br style="{{ .Args }}">
	<script>
		document.getElementById("{{.Name}}").style.border="none";
	</script>
`))

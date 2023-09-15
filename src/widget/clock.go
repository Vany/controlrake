package widget

import (
	"context"
	"fmt"
	"github.com/vany/controlrake/src/cont"
	"html/template"
	"io"
	"time"
)

type Clock struct {
	BaseWidget
	Format string
}

var _ = MustSurvive(RegisterWidgetType(&Clock{}))

func (w *Clock) Init(ctx context.Context) error {
	done := ctx.Done()
	if s, ok := w.Args.(string); !ok {
		return w.Errorf("can's read args: %v", w.Args)
	} else {
		w.Format = s
	}
	go func() {
	STOP:
		for {
			select {
			case t := <-time.Tick(time.Second):
				w.Send(t.Format(w.Format))
			case <-done:
				cont.FromContext(ctx).Log.Info().Msg("Clock shut down")
				break STOP
			}
		}
	}()

	return nil
}

func (w *Clock) RenderTo(wr io.Writer) error {
	if err := TClock.Execute(wr, w); err != nil {
		return fmt.Errorf("render failed: %w", err)
	}
	return nil
}

var TClock = template.Must(template.New("Label").Parse(`
<div class="widget" id="{{.Name}}">
	<b>🕰️</b>	
	
	<script>
		console.log("I'm here")
		let self = document.getElementById("{{.Name}}")
		self.onWSEvent = function (msg) {
			self.getElementsByTagName("b")[0].innerHTML = msg
		}
	</script>
</div>
`))

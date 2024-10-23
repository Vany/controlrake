package oldwidgets

import (
	"context"
	"github.com/vany/controlrake/src/widget/impl"
	"time"
)

type Clock struct {
	impl.BaseWidget
	Format string
}

var _ = MustSurvive(impl.RegisterWidgetType(&Clock{}, `
	<b>🕰️</b>	
	<script>
		let self = document.getElementById("{{.Name}}")
		self.onWSEvent = function (msg) {
			self.getElementsByTagName("b")[0].innerHTML = msg
		}
	</script>
`))

func (w *Clock) Init(ctx context.Context) error {
	if s, ok := w.Args.(string); !ok {
		return w.Errorf("can's read args: %v", w.Args)
	} else {
		w.Format = s
	}
	go func() {
		for {
			w.Send((<-time.Tick(time.Second)).Format(w.Format))
			if ctx.Err() != nil {
				w.Log.Info().Msg("Clock shut down")
				break
			}
		}
	}()

	return nil
}

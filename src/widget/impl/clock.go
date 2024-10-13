package impl

import (
	"github.com/vany/controlrake/src/widget/api"
	"golang.org/x/net/context"
	"time"
)

// Clock - was for outbound messages development
type Clock struct {
	BaseWidget
	Format string
}

var _ = RegisterWidgetType(&Clock{}, `
	<pre>🕰️</pre>	
	<script>
		let self = document.getElementById("{{.Name}}")
		self.onWSEvent = function (msg) {
			self.getElementsByTagName("pre")[0].innerHTML = msg
		}
	</script>
`)

func (w *Clock) Init(ctx context.Context, _ api.WidgetConstructor) error {
	if w.Args == nil {
		w.Format = "15:04:05"
	} else if s, ok := w.Args.(string); !ok {
		return w.Errorf("can's read args: %v", w.Args)
	} else {
		w.Format = s
	}
	go func() {
		for {
			w.SendToWeb(ctx, (<-time.Tick(time.Second)).Format(w.Format))
			if ctx.Err() != nil {
				break
			}
		}
		w.Log.Debug().Msg("Clock shut down")
	}()

	return nil
}

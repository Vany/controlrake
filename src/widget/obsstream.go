package widget

import (
	"context"
	"fmt"
	"github.com/andreykaipov/goobs/api/requests/stream"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/obs"
	"strings"
	"time"

	. "github.com/vany/pirog"
)

type ObsStream struct {
	BaseWidget
}

var _ = MustSurvive(RegisterWidgetType(&ObsStream{}, `
	<span></span>
	<span></span>
	<span></span>

	<script>
		console.log("I'm OBS Stream");
		let self = document.getElementById("{{.Name}}");
		self.onWSEvent = function (msg) {
			const inf = JSON.parse(msg);
			let [symbol, length, congestion] = self.getElementsByTagName("span") 		
			symbol.innerHTML = inf.Active ? (inf.Reconnect ? "🟨" : "🟩") : "🟥";
			length.innerHTML = inf.Length;
			congestion.innerHTML = inf.Congestion;
		}
	</script>
`))

type ObsStreamInfo struct {
	Active     bool
	Reconnect  bool
	Congestion float64
	Length     string
}

// todo decide how to show skipped frames . Δframes ?

func (w *ObsStream) Init(ctx context.Context) error {
	done := ctx.Done()
	app := app.FromContext(ctx)
	go func() {
	STOP:
		for {
			select {
			case <-time.Tick(time.Second):
				o := app.Obs.(*obs.Obs)
				inf, err := obs.Wrapper(ctx, o, func() (*stream.GetStreamStatusResponse, error) {
					return o.Client.Stream.GetStreamStatus()
				})

				if err != nil {
					w.Log.Error().Err(err).Msg("GetStreamStatus() failed")
				} else {
					tarr := strings.SplitN(inf.OutputTimecode, ":", 3)
					w.Send(ToJson(ObsStreamInfo{
						Active:     inf.OutputActive,
						Reconnect:  inf.OutputReconnecting,
						Length:     fmt.Sprintf("%s:%s", tarr[0], tarr[1]),
						Congestion: inf.OutputCongestion,
					}))
				}
			case <-done:
				w.Log.Info().Msg("shut down")
				break STOP
			}
		}
	}()

	return nil
}

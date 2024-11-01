package impl

import (
	"fmt"
	"github.com/andreykaipov/goobs"
	"github.com/andreykaipov/goobs/api/events"
	"github.com/andreykaipov/goobs/api/requests/stream"
	obs_api "github.com/vany/controlrake/src/obs/api"
	"github.com/vany/controlrake/src/widget/api"
	"github.com/vany/pirog"
	"golang.org/x/net/context"
	"reflect"
	"strings"
	"time"
)

type ObsStream struct {
	BaseWidget
	Obs obs_api.Obs
}

var _ = RegisterWidgetType(&ObsStream{}, `
<div style="display: inline-flex">
	<span>⬜️</span> <span>0</span>=<span>0</span>

	<script>
		let self = document.getElementById("{{.Name}}");
		self.onWSEvent = function (msg) {
			const inf = JSON.parse(msg);
			let [symbol, length, congestion] = self.getElementsByTagName("span") 		
			symbol.innerHTML = inf.Active ? (inf.Reconnect ? "🟨" : "🟩") : "🟥";
			length.innerHTML = inf.Length;
			congestion.innerHTML = inf.Congestion;
		}
	</script>
</div>
`)

type ObsStreamInfo struct {
	Active     bool
	Reconnect  bool
	Congestion float64
	Length     string
}

func (w *ObsStream) Init(ctx context.Context, c api.WidgetConstructor) error {
	w.Obs = c.GetComponent("Obs").(obs_api.Obs)
	on := w.Obs.AvailabilityNotification().Subscribe(true)
	off := w.Obs.AvailabilityNotification().Subscribe(false)
	strChanged := w.Obs.EventStream().Subscribe(reflect.TypeOf(&events.StreamStateChanged{}))
	done := ctx.Done()
	go func() {
		active := false
		for {
			select {
			case <-on:
				w.SendToWeb(ctx, `{"enabled": true}`)
				w.Log.Debug().Msg("ON")
			case <-off:
				w.SendToWeb(ctx, `{"enabled": false}`)

			case a := <-strChanged:
				if rsc, ok := a.(*events.StreamStateChanged); !ok {
					w.Log.Error().Interface("data", a).Msg("FIGNYA")
				} else {
					active = rsc.OutputActive
					if !active {
						w.SendToWeb(ctx, pirog.ToJson(ObsStreamInfo{
							Active:     false,
							Reconnect:  false,
							Length:     "0",
							Congestion: 0,
						}))

					}
				}

			case <-time.Tick(time.Second / 2):
				if active {
					o := w.Obs
					inf := &stream.GetStreamStatusResponse{}
					if err := o.Execute(func(o *goobs.Client) (err error) {
						inf, err = o.Stream.GetStreamStatus()
						return err
					}); err != nil {
						w.Log.Error().Err(err).Msg("GetStreamStatus() failed")
					} else {
						tarr := strings.SplitN(inf.OutputTimecode, ":", 3)
						w.SendToWeb(ctx, pirog.ToJson(ObsStreamInfo{
							Active:     inf.OutputActive,
							Reconnect:  inf.OutputReconnecting,
							Length:     fmt.Sprintf("%s:%s", tarr[0], tarr[1]),
							Congestion: inf.OutputCongestion,
						}))
					}
				}
			case <-done:
				w.Log.Info().Msg("shut down")
				return
			}
		}
	}()

	return nil
}

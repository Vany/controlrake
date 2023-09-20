package widget

import (
	"context"
	"fmt"
	"github.com/andreykaipov/goobs/api/requests/record"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/obs"
	. "github.com/vany/pirog"
	"strings"
	"time"
)

type ObsRecord struct {
	BaseWidget
}

var _ = MustSurvive(RegisterWidgetType(&ObsRecord{}, `
	<button onclick="Send(this, 'rec')">⏺️</button>
	<span></span>
	<button onclick="Send(this, 'pause')">⏸️</button>

	<script>
		console.log("I'm OBS")
		let self = document.getElementById("{{.Name}}")
		self.onWSEvent = function (msg) {
			const inf = JSON.parse(msg);
			self.getElementsByTagName("span")[0].innerHTML = inf.Length;
			let [brec, bpause] = self.getElementsByTagName("button");
			brec.innerHTML =  inf.Rec ? "🅾️" : "⏺️";
			bpause.innerHTML = inf.Rec && inf.Pause ? "♊️" : "⏸️" ;
		}
	</script>
`))

type ObsRecordInfo struct {
	Rec    bool
	Pause  bool
	Length string
}

func (w *ObsRecord) Init(ctx context.Context) error {
	done := ctx.Done()
	con := app.FromContext(ctx)
	go func() {
	STOP:
		for {
			select {
			case <-time.Tick(time.Second):
				o := con.Obs.(*obs.Obs)
				inf, err := obs.Wrapper(ctx, o, func() (*record.GetRecordStatusResponse, error) {
					return o.Client.Record.GetRecordStatus()
				})
				if err != nil {
					con.Log.Error().Err(err).Msg("obs failed to GetRecordStatus()")
				}
				tarr := strings.SplitN(inf.OutputTimecode, ":", 3)
				var length string
				if len(tarr) > 1 {
					length = fmt.Sprintf("%s:%s", tarr[0], tarr[1])
				} else if len(tarr) > 0 {
					length = "timecode=" + inf.OutputTimecode
				} else {
					length = "timecode empty"
				}
				w.Send(ToJson(ObsRecordInfo{
					Rec:    inf.OutputActive,
					Pause:  inf.OutputPaused,
					Length: length,
				}))
			case <-done:
				con.Log.Info().Msg("Clock shut down")
				break STOP
			}
		}
	}()

	return nil
}

func (w *ObsRecord) Consume(ctx context.Context, event []byte) error {
	con := app.FromContext(ctx)
	o := con.Obs.(*obs.Obs)
	con.Log.Log().Bytes("event", event).Msg("Pressed")
	var err error

	switch string(event) {
	case "rec":
		_, err = obs.Wrapper(ctx, o, func() (*record.ToggleRecordResponse, error) {
			return o.Client.Record.ToggleRecord()
		})

	case "pause":
		_, err = obs.Wrapper(ctx, o, func() (*record.ToggleRecordPauseResponse, error) {
			return o.Client.Record.ToggleRecordPause()
		})

	default:
		err = fmt.Errorf("unknown event %s", event)
	}

	return err
}

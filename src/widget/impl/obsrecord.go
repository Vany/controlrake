package impl

import (
	"fmt"
	"github.com/andreykaipov/goobs"
	"github.com/andreykaipov/goobs/api/requests/record"
	obs_api "github.com/vany/controlrake/src/obs/api"
	"github.com/vany/controlrake/src/widget/api"
	"github.com/vany/pirog"
	"golang.org/x/net/context"
	"strings"
	"time"
)

// ObsRecord - Control record via obs
type ObsRecord struct {
	BaseWidget
	Obs obs_api.Obs
}

var _ = RegisterWidgetType(&ObsRecord{}, `
<div style="display: inline-flex; font-size: xx-large">

	<button onclick="Send(this, 'rec')">⏺️</button>
	<span></span>
	<button onclick="Send(this, 'pause')">⏸️</button>

	<script>
		let self = document.getElementById("{{.Name}}")
		self.onWSEvent = function (msg) {
			const inf = JSON.parse(msg);
			self.getElementsByTagName("span")[0].innerHTML = inf.Length;
			let [brec, bpause] = self.getElementsByTagName("button");
			brec.innerHTML =  inf.Rec ? "🅾️" : "⏺️";
			bpause.innerHTML = inf.Rec && inf.Pause ? "♊️" : "⏸️" ;
		}
	</script>
</div>
`)

type ObsRecordInfo struct {
	Rec    bool
	Pause  bool
	Length string
}

func (w *ObsRecord) Init(ctx context.Context, c api.WidgetConstructor) error {
	w.Obs = c.GetComponent("Obs").(obs_api.Obs)
	done := ctx.Done()
	go func() {
		for {
			select {
			// TODO: do not be that fast if not working and in stream too
			case <-time.Tick(time.Second / 2):
				inf := &record.GetRecordStatusResponse{}
				if err := w.Obs.Execute(func(o *goobs.Client) (err error) {
					inf, err = o.Record.GetRecordStatus()
					return err
				}); err != nil {
					w.Log.Error().Err(err).Msg("obs failed to GetRecordStatus()")
					continue
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
				w.SendToWeb(ctx, pirog.ToJson(ObsRecordInfo{
					Rec:    inf.OutputActive,
					Pause:  inf.OutputPaused,
					Length: length,
				}))
			case <-done:
				return
			}
		}
	}()

	return nil
}

func (w *ObsRecord) Dispatch(ctx context.Context, event string) (err error) {
	w.Log.Log().Str("event", event).Msg("Pressed")

	switch event {
	case "rec":
		w.Obs.Execute(func(o *goobs.Client) error {
			_, err = o.Record.ToggleRecord()
			return err
		})

	case "pause":
		w.Obs.Execute(func(o *goobs.Client) error {
			_, err = o.Record.ToggleRecordPause()
			return err
		})

	default:
		err = fmt.Errorf("unknown event %s", event)
	}

	return err
}

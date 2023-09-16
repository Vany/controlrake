package widget

import (
	"context"
	"fmt"
	"github.com/andreykaipov/goobs/api/requests/record"
	"github.com/vany/controlrake/src/cont"
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
	con := cont.FromContext(ctx)
	go func() {
	STOP:
		for {
			select {
			case <-time.Tick(time.Second):
				var inf *record.GetRecordStatusResponse
				con.Obs.Transaction(func() { inf = con.Obs.InfoRecord(ctx) })
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
	con := cont.FromContext(ctx)
	con.Log.Log().Bytes("event", event).Msg("Pressed")
	var err error
	con.Obs.Transaction(func() {
		switch string(event) {
		case "rec":
			_, err = con.Obs.Cli().Record.ToggleRecord()
		case "pause":
			_, err = con.Obs.Cli().Record.ToggleRecordPause()
		default:
			err = fmt.Errorf("unknown event %s", event)
		}
	})
	return err
}

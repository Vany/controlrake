package widget

import (
	"context"
	"github.com/creasty/defaults"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/youtube"
	. "github.com/vany/pirog"
	"html/template"
	"time"
)

type Youtube struct {
	BaseWidget
	StartChan chan struct{}
	PerPage   int `default:"10"`
	Period    int `default:"2"`
}

var _ = MustSurvive(RegisterWidgetType(&Youtube{}, `
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
CHATCHATCHATCHATCHATCHATCHATCHATC
<span></span>
<br>
<div style="overflow-y: scroll; font-size: 40%; height: 90%;">
</div>

<script>
	let self = document.getElementById("{{.Name}}");
	Send(self,"load");

	self.onWSEvent = function (msg) {
		const [inf, msgs] = JSON.parse(msg);	
		let sel = self.getElementsByTagName("span")[0];
		sel.innerHTML = inf.statistics.concurrentViewers + " " + inf.status.lifeCycleStatus;
		let div = self.getElementsByTagName("div")[0];
		div.innerHTML = "";
		msgs.forEach((v) => {div.innerHTML += "<br>" + v});
	};
	
</script>
`))

func (w *Youtube) Init(ctx context.Context) error {
	defaults.MustSet(w)
	mapstructure.Decode(w.BaseWidget.Config.Args, w)
	w.StartChan = make(chan struct{})

	go func() {
		<-w.StartChan
		app := app.FromContext(ctx)
		for !app.Youtube.Ready() {
			<-time.After(time.Second)
		}
		w.Log.Info().Msg("Youtube component found")
		cc := app.Youtube.(*youtube.Youtube).GetChatConnection(ctx, w.PerPage)
		for {
			w.Send(ToJson([]any{cc.Info, MAP(cc.Spin(ctx), template.HTMLEscapeString)}))
			<-time.After(5 * time.Second)
			if ctx.Err() != nil {
				return
			}
		}
	}()

	return nil
}

func (w *Youtube) Dispatch(ctx context.Context, event string) error {
	w.Log.Debug().Msg("Youtube loading")
	if event == "load" && w.StartChan != nil {
		close(w.StartChan)
		w.StartChan = nil
	}
	return nil
}

// todo react for load

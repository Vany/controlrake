package widget

import (
	"context"
	app2 "github.com/vany/controlrake/src/app"
)

var _ = MustSurvive(RegisterWidgetType(&Eval{}, `
<input><br>
<span></span>

<script>
		let self = document.getElementById({{.Name}});
		self.getElementsByTagName("input")[0].onkeyup = function (ev) {
			if (ev.key == "Enter") Send(self, this.value);		
		};
		
		self.onWSEvent = function (msg) {
			self.getElementsByTagName("span")[0].innerHTML = msg
		}

</script>
`))

type Eval struct {
	BaseWidget
}

func (w *Eval) Dispatch(ctx context.Context, event []byte) error {
	app := app2.FromContext(ctx)
	go func() {
		so := app.ObsBrowser.Send(ctx, "Eval|"+string(event))
		ret := <-so.Receive()
		app.Log.Info().Str("ret", ret).Msg("Eval received")
		w.Send(ret)
		<-so.Done()
		app.Log.Info().Msg("Eval done")

	}()
	return nil
}

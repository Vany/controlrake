package widget

import (
	"context"
	"github.com/vany/controlrake/src/app"
)

var _ = MustSurvive(RegisterWidgetType(&Eval{}, `
<input><br>
<span></span>

<script>
		let self = document.getElementById("{{.Name}}");
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

func (w *Eval) Dispatch(ctx context.Context, event string) error {
	app := app.FromContext(ctx)
	go func() {
		so := app.ObsBrowser.Send(ctx, "Eval|"+string(event))
		ret := <-so.Receive()
		w.Log.Info().Str("ret", ret).Msg("Eval received")
		w.Send(ret)
		<-so.Done()
		w.Log.Info().Msg("Eval done")

	}()
	return nil
}

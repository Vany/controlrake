package oldwidgets

import (
	"github.com/vany/controlrake/src/widget/impl"
	"golang.org/x/net/context"
)

var _ = impl.RegisterWidgetType(&Eval{}, `
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
`)

type Eval struct {
	impl.BaseWidget
}

func (w *Eval) Dispatch(ctx context.Context, event string) error {
	go func() {
		so := w.ObsBrowser.Send(ctx, "Eval|"+string(event))
		ret := <-so.Receive()
		w.Log.Info().Str("ret", ret).Msg("Eval received")
		w.Send(ret)
		<-so.Done()
		w.Log.Info().Msg("Eval done")
	}()
	return nil
}

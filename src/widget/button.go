package widget

import (
	"context"
	"fmt"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
)

type ButtonArgs struct {
	Action *struct {
		PlaySound   string
		File        string
		Html        string
		CommandLine string
	}
}

type Button struct {
	BaseWidget
	Args ButtonArgs
}

var _ = MustSurvive(RegisterWidgetType(&Button{}, `
<button>{{.Caption}}</button>

<script>
	let self = document.getElementById({{.Name}})
	{{if not .Args.Action }} // button have an action
	self.onclick = function() {
		Send(this,"click")
	};
	{{else}}
	self.onclick = function() {
		Send(this,"click");
		self.style.background = "#00ff00";
	};

	{{UnEscape .Name}}_Background = self.style.background;

	self.onWSEvent = function (msg) {
		if (msg == "done") return self.style.background = {{UnEscape .Name}}_Background;
		// msg float from 0 to 1
		saturation =  Math.round(0xff * msg)    // 0 -> ff
		
		let bgcolor = "#" + saturation.toString(16).padStart(2, "0") + "ff" + saturation.toString(16).padStart(2, "0"); 
		self.style.background = bgcolor;
		console.log(msg + " => " + bgcolor);
	}
		
	{{end}}
	
	function {{UnEscape .Name}}_Click() {
			self.bgColor = ""
			
	}
		
</script>
`))

func (w *Button) Init(context.Context) error {
	err := mapstructure.Decode(w.Config.Args, &w.Args)
	return TERNARY(err == nil, nil, w.Errorf("cant read config %#v: %w", w.Config.Args, err))
}

// TODO 🔴REFACTOR!!!!🔴  yes, we can!!!🟢
func (w *Button) Dispatch(ctx context.Context, event []byte) error {
	app := app.FromContext(ctx)
	app.Log.Log().Bytes("event", event).Msg("Pressed")

	if w.Args.Action == nil {
		return w.Errorf(".Action is nil")
	}

	if w.Args.Action.PlaySound != "" {
		sendObj := app.ObsBrowser.Send(ctx, "PlaySound|"+w.Args.Action.PlaySound)
		go func() {
			for {
				select {
				case msg := <-sendObj.Receive():
					app.Log.Debug().Str("msg", msg).Msg("WS Got")
					w.Send(msg)
				case <-sendObj.Done():
					app.Log.Debug().Msg("WS Closed")
					w.Send("done")
					return
				case <-ctx.Done():
					return
				}
			}
		}()
	}

	if w.Args.Action.Html != "" {
		sendObj := app.ObsBrowser.Send(ctx, fmt.Sprintf("Html|%s", w.Args.Action.Html))
		go func() {
			for {
				select {
				case msg := <-sendObj.Receive():
					app.Log.Debug().Str("msg", msg).Msg("WS Got")
					w.Send(msg)
				case <-sendObj.Done():
					app.Log.Debug().Msg("WS Closed")
					w.Send("done")
					return
				case <-ctx.Done():
					return
				}
			}
		}()
	}

	return nil
}

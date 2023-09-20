package widget

import (
	"context"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
)

type ButtonArgs struct {
	Action string
	Sound  string
}

type Button struct {
	BaseWidget
	Args ButtonArgs
}

var _ = MustSurvive(RegisterWidgetType(&Button{}, `
<button>{{.Caption}}</button>

<script>
	let self = document.getElementById({{.Name}})
	self.onclick = function() {
		Send(this,"click")	
	};
	{{if ne .Args.Action "" }} // button have an action
	self.onclick = function() {
		Send(this,"click");
		self.style.background = "#00ff00";
	};
	{{UnEscape .Name}}_Background = self.style.background;
	self.onWSEvent = function (msg) {
		if (msg == "done") return self.style.background = {{UnEscape .Name}}_Background;
		// msg float from 0 to 1
		saturation =  Math.round(0x77 * msg)    // 0 -> 77
		self.style.background = "#" + saturation.toString(16) + (0xff - saturation).toString(16) + saturation.toString(16); 

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

func (w *Button) Consume(ctx context.Context, event []byte) error {
	app := app.FromContext(ctx)
	app.Log.Log().Bytes("event", event).Msg("Pressed")

	if w.Args.Sound != "" {

	} else if w.Args.Action != "" {
		sendObj := app.ObsBrowser.Send(ctx, w.Args.Action)
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

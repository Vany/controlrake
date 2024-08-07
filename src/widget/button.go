package widget

import (
	"bufio"
	"context"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
	"os/exec"
	"strings"
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
<button style="font-size: xx-large">{{.Caption}}</button>

<script>
	let self = document.getElementById("{{.Name}}");
	{{if not .Args.Action }} // button have an action
	self.onclick = function() {
		Send(this,"click")
	};
	{{else}}
	
	let bu = self.getElementsByTagName("button")[0];
	self.OldBackground = self.style.background;
	self.Bu_OldBackground = bu.style.background;
	
	self.onclick = function() {
		Send(this,"click");
		self.style.background = "#00ff00";
		bu.style.background="transparent";
	};


	self.onWSEvent = function (msg) {
		let [event, data] = msg.split("|", 2);
		if (msg == "done") {
			self.style.background = self.OldBackground;
			bu.style.background = self.Bu_OldBackground;
		} else if (event == "progress") {
			let saturation =  Math.round(0xff * data);
			bu.style.background = "#" + saturation.toString(16).padStart(2, "0") + "ff" + saturation.toString(16).padStart(2, "0"); 			 
		} else if (event == "out") {
			console.log("CMD: " + data);
		} 
	}
		
	{{end}}
	
	function {{.Name}}_Click() {
			self.bgColor = ""
			
	}
		
</script>
`))

func (w *Button) Init(context.Context) error {
	err := mapstructure.Decode(w.Config.Args, &w.Args)
	return TERNARY(err == nil, nil, w.Errorf("cant read config %#v: %w", w.Config.Args, err))
}

// TODO 🔴REFACTOR!!!!🔴  yes, we can!!!🟢
func (w *Button) Dispatch(ctx context.Context, event string) error {
	app := app.FromContext(ctx)
	w.Log.Log().Str("event", event).Msg("Pressed")

	if w.Args.Action == nil {
		return w.Errorf(".Action is nil")
	}

	if w.Args.Action.PlaySound != "" {
		sendObj := app.ObsBrowser.Send(ctx, "PlaySound|"+w.Args.Action.PlaySound)
		go func() {
			for {
				select {
				case msg := <-sendObj.Receive():
					w.Log.Debug().Str("msg", msg).Msg("WS Got")
					w.Send("progress|" + msg)
				case <-sendObj.Done():
					w.Log.Debug().Msg("WS Closed")
					w.Send("done")
					return
				case <-ctx.Done():
					return
				}
			}
		}()
	}

	if w.Args.Action.Html != "" {
		sendObj := app.ObsBrowser.Send(ctx, "Html|"+w.Args.Action.Html)
		go func() {
			for {
				select {
				case msg := <-sendObj.Receive():
					w.Log.Debug().Str("msg", msg).Msg("WS Got")
					w.Send("progress|" + msg)
				case <-sendObj.Done():
					w.Log.Debug().Msg("WS Closed")
					w.Send("done")
					return
				case <-ctx.Done():
					return
				}
			}
		}()
	}

	if w.Args.Action.CommandLine != "" {

		go func() {
			args := strings.Split(w.Args.Action.CommandLine, " ")
			cmd := exec.CommandContext(ctx, args[0], args[1:]...)
			stdout, err := cmd.StdoutPipe()
			if err != nil {
				w.Send("cmderror|" + err.Error())
				w.Log.Error().Err(err).Send()
				return
			}

			if err := cmd.Start(); err != nil {
				w.Send("cmderror|" + err.Error())
				w.Log.Error().Err(err).Send()
				return
			}

			scanner := bufio.NewScanner(stdout)
			for scanner.Scan() {
				w.Send("out|" + scanner.Text())
			}

			if err := cmd.Wait(); err != nil {
				w.Send("cmderror|" + err.Error())
				w.Log.Error().Err(err).Send()
			}
			w.Send("done")
		}()
	}

	return nil
}

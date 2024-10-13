package impl

import (
	"github.com/mitchellh/mapstructure"
	obsbrowser_api "github.com/vany/controlrake/src/obsbrowser/api"
	"github.com/vany/controlrake/src/widget/api"
	"github.com/vany/pirog"
	"golang.org/x/net/context"
	"html/template"
)

type ButtonArgs struct {
	Action *struct {
		PlaySound   string
		File        string
		Html        string
		CommandLine string
	}
}

// Button - generic button that can invoke actions and show action progress
type Button struct {
	BaseWidget
	Args ButtonArgs

	ObsBrowser obsbrowser_api.ObsBrowser
}

var _ = RegisterWidgetType(&Button{}, `
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
	
	function {{ .RawName }}_Click() {
			self.bgColor = ""		
	}
		
</script>
`)

func (w *Button) Init(_ context.Context, c api.WidgetConstructor) error {
	w.ObsBrowser = c.GetComponent("ObsBrowser").(obsbrowser_api.ObsBrowser)
	err := mapstructure.Decode(w.WidgetConfig.Args, &w.Args)
	return pirog.TERNARY(err == nil, nil, w.Errorf("cant read config %#v: %w", w.WidgetConfig.Args, err))

}

func (w *Button) Dispatch(ctx context.Context, event string) error {
	w.Log.Log().Str("event", event).Msg("Pressed")

	if w.Args.Action.PlaySound != "" {
		req := w.ObsBrowser.ToWeb(ctx, "PlaySound|"+w.Args.Action.PlaySound)

		go func() {
			for {
				if msg, ok := pirog.RECV(ctx, req.RES); !ok {
					return
				} else {
					w.Log.Debug().Str("msg", msg).Msg("OBSWS response")
					w.SendToWeb(ctx, "progress|"+msg)
				}
			}
		}()
	}

	return nil
}

func (w *Button) RawName() template.JS { return template.JS(w.Name) }

/*
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
*/

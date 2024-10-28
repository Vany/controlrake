package impl

import (
	"context"
	"fmt"
	"github.com/andreykaipov/goobs"
	"github.com/andreykaipov/goobs/api/requests/inputs"
	"github.com/mitchellh/mapstructure"
	obs_api "github.com/vany/controlrake/src/obs/api"
	"github.com/vany/controlrake/src/widget/api"
	"github.com/vany/pirog"
	"strings"
)

type ObsInputsArgs struct {
	InputName string
	Property  string
	List      []string
}

// ObsInputs - switch config input device of specified capturer
type ObsInputs struct {
	BaseWidget
	Args ObsInputsArgs
	Obs  obs_api.Obs
}

var _ = RegisterWidgetType(&ObsInputs{}, `
<select style="font-size: xx-large"></select>
<script>
	let self = document.getElementById("{{.Name}}");
	let sel = self.getElementsByTagName("select")[0];
	Send(self,"load");

	self.onWSEvent = function (msg) {
		const inf = JSON.parse(msg);	
		if ('enabled' in inf) {		
			console.log("OBSInputs", inf.enabled);
			if (inf.enabled) {
				sel.disabled = false;
				Send(self,"load");
			} else {
				sel.disabled = true;
			}
			return;
		}
		
		sel.innerHTML = "";
		inf.List.forEach(s => {
			let selected = (inf.Selected == s ? "selected" : ""); 
			sel.innerHTML +=  '<option ' + selected + '>' + s + "</option>";
			; 		
		});
		sel.style.backgroundColor = "white"
	};
	
	self.onchange = function(e) {
		sel.style.backgroundColor = "red"
		Send(this,  "set|" +  e.target.value )
	};
	
</script>
`)

func (w *ObsInputs) Init(ctx context.Context, c api.WidgetConstructor) error {
	w.Obs = c.GetComponent("Obs").(obs_api.Obs)
	if err := mapstructure.Decode(w.WidgetConfig.Args, &w.Args); err != nil {
		return w.Errorf("cant read config %#v: %w", w.WidgetConfig.Args, err)
	}

	go func() {
		on := w.Obs.AvailabilityNotification().Subscribe(true)
		off := w.Obs.AvailabilityNotification().Subscribe(false)
		for {
			select {
			case <-on:
				w.SendToWeb(ctx, `{"enabled": true}`)
				w.Log.Debug().Msg("ON")

				go w.SendInputs(ctx)
			case <-off:
				w.SendToWeb(ctx, `{"enabled": false}`)
			case <-ctx.Done():
				w.Obs.AvailabilityNotification().UnSubscribe(true, on)
				w.Obs.AvailabilityNotification().UnSubscribe(false, off)
				return
			}
		}

	}()

	return nil
}

func (w *ObsInputs) Dispatch(ctx context.Context, event string) error {
	w.Log.Debug().Str("event", event).Msg("Pressed")

	if device, ok := strings.CutPrefix(event, "set|"); ok {
		if err := w.Obs.Execute(func(o *goobs.Client) (err error) {
			_, err = o.Inputs.SetInputSettings(&inputs.SetInputSettingsParams{
				InputName:     &w.Args.InputName,
				InputSettings: map[string]any{"device_name": device},
			})
			return err
		}); err != nil {
			w.Log.Error().Str("event", event).Err(err).Msg("SetInputSettings()")
		} else if err := w.SendInputs(ctx); err != nil {
			w.Log.Error().Str("event", event).Err(err).Msg("w.SendInputs()")
		}

	} else if event == "load" { // TODO redundant
		w.SendInputs(ctx)

	} else {
		w.Log.Error().Str("event", event).Msg("wtf")
	}

	return nil
}

// SendScene - scenelist -> web
func (w *ObsInputs) SendInputs(ctx context.Context) error {
	res := &inputs.GetInputSettingsResponse{}
	if err := w.Obs.Execute(func(o *goobs.Client) (err error) {
		res, err = o.Inputs.GetInputSettings(&inputs.GetInputSettingsParams{InputName: &w.Args.InputName})
		return err
	}); err != nil {
		return fmt.Errorf("GetInputSettings(%s) failed: %w", w.Args.InputName, err)
	} else {
		w.SendToWeb(ctx, pirog.ToJson(struct {
			Selected any
			List     []string
		}{res.InputSettings["device_name"], w.Args.List}))
	}

	return nil
}

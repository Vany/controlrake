package impl

import (
	"context"
	"fmt"
	"strings"

	"github.com/andreykaipov/goobs"
	"github.com/andreykaipov/goobs/api/requests/scenes"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/pirog"

	obs_api "github.com/vany/controlrake/src/obs/api"
	"github.com/vany/controlrake/src/widget/api"
)

type Scenes struct {
	BaseWidget
	Obs obs_api.Obs
}

var _ = RegisterWidgetType(&Scenes{}, `
<select style="font-size: xx-large"></select>
<script>
	let self = document.getElementById("{{.Name}}");
	let sel = self.getElementsByTagName("select")[0];
	Send(self,"load");

	self.onWSEvent = function (msg) {
		const inf = JSON.parse(msg);	
		if ('enabled' in inf) {		
			console.log("OBS", inf.enabled);
			if (inf.enabled) {
				sel.disabled = false;
				Send(self,"load");
			} else {
				sel.disabled = true;
			}
			return;
		}
		
		sel.innerHTML = "";
		inf.scenes.forEach(s => {
			let selected = (inf.currentProgramSceneName == s.sceneName ? "selected" : ""); 
			let xxx = '<option ' + selected + '>' + s.sceneName + "</option>";
			sel.innerHTML += xxx; 		
		});
		sel.style.backgroundColor = "white"
	};
	
	self.onchange = function(e) {
		sel.style.backgroundColor = "red"
		Send(this,  "set|" +  e.target.value )
	};
	
</script>
`)

func (w *Scenes) Init(ctx context.Context, c api.WidgetConstructor) error {
	w.Obs = c.GetComponent("Obs").(obs_api.Obs)
	if err := mapstructure.Decode(w.WidgetConfig.Args, &w.Args); err != nil {
		return w.Errorf("cant read config %#v: %w", w.WidgetConfig.Args, err)
	}

	go func() {
		on := w.Obs.AvailabilityNotification().Subscribe(true)
		off := w.Obs.AvailabilityNotification().Subscribe(false)
		all := w.Obs.EventStream().Subscribe(obs_api.AllEvents)
		for {
			select {
			case e := <-all:
				fmt.Printf("%T\n", e)
			case <-on:
				w.SendToWeb(ctx, `{"enabled": true}`)
				w.SendScene(ctx)
				w.Log.Log().Msg("TRUE")
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

func (w *Scenes) Dispatch(ctx context.Context, event string) error {
	w.Log.Log().Str("event", event).Msg("Pressed")

	if sc, ok := strings.CutPrefix(event, "set|"); ok {
		if err := w.Obs.Execute(func(o *goobs.Client) (err error) {
			_, err = o.Scenes.SetCurrentProgramScene(&scenes.SetCurrentProgramSceneParams{SceneName: &sc})
			return err
		}); err != nil {
			w.Log.Error().Str("event", event).Err(err).Msg("SetCurrentProgramScene()")
		} else if err := w.SendScene(ctx); err != nil {
			w.Log.Error().Str("event", event).Err(err).Msg("w.SendScene()")
		}

	} else if event == "load" {
		w.SendScene(ctx)

	} else {
		w.Log.Error().Str("event", event).Msg("wtf")
	}

	return nil
}

// SendScene - scenelist -> web
func (w *Scenes) SendScene(ctx context.Context) error {
	res := &scenes.GetSceneListResponse{}
	if err := w.Obs.Execute(func(o *goobs.Client) (err error) {
		res, err = o.Scenes.GetSceneList()
		return err
	}); err != nil {
		return fmt.Errorf("GetSceneList() failed: %w", err)
	}
	if res.Scenes != nil { // workarround bug in lib, some time err==nil
		w.SendToWeb(ctx, pirog.ToJson(res))
	}

	return nil
}

package oldwidgets

import (
	"github.com/vany/controlrake/src/widget/impl"
)

type Scenes struct {
	impl.BaseWidget
}

var _ = MustSurvive(impl.RegisterWidgetType(&Scenes{}, `
<select style="font-size: xx-large"></select>
<script>
	let self = document.getElementById("{{.Name}}");
	Send(self,"load");

	self.onWSEvent = function (msg) {
		const inf = JSON.parse(msg);	
		let sel = self.getElementsByTagName("select")[0];
		sel.innerHTML = "";
		inf.scenes.forEach(s => {
			let selected = (inf.currentProgramSceneName == s.sceneName ? "selected" : ""); 
			let xxx = '<option ' + selected + '>' + s.sceneName + "</option>";
			sel.innerHTML += xxx; 		
		});
		self.style.backgroundColor  = "white"
	};
	
	self.onchange = function(e) {
		self.style.backgroundColor  = "red";
		Send(this,  "set|" +  e.target.value )
	};
	
	
</script>
`))

//func (w *Scenes) Init(ctx context.Context) error {
//	err := mapstructure.Decode(w.Config.Args, &w.Args)
//	return pirog.TERNARY(err == nil, nil, w.Errorf("cant read config %#v: %w", w.Config.Args, err))
//}
//
//func (w *Scenes) Dispatch(ctx context.Context, event string) error {
//	app := app.FromContext(ctx)
//	w.Log.Log().Str("event", event).Msg("Pressed")
//	o := app.Obs.(*obs.Obs)
//
//	if sc, ok := strings.CutPrefix(event, "set|"); ok {
//		if _, err := obs.Wrapper(ctx, o, func() (*scenes.SetCurrentProgramSceneResponse, error) {
//			return o.Client.Scenes.SetCurrentProgramScene(&scenes.SetCurrentProgramSceneParams{SceneName: &sc})
//		}); err != nil {
//			w.Log.Error().Str("event", event).Err(err).Msg("SetCurrentProgramScene()")
//		}
//		if err := w.SendScene(ctx, o); err != nil {
//			w.Log.Error().Str("event", event).Err(err).Msg("w.SendScene()")
//		}
//
//	} else if event == "load" {
//		w.SendScene(ctx, o)
//
//	} else {
//		w.Log.Error().Str("event", event).Msg("wtf")
//	}
//
//	return nil
//}
//
//func (w *Scenes) SendScene(ctx context.Context, o *obs.Obs) error {
//	if ret, err := obs.Wrapper(ctx, o, func() (*scenes.GetSceneListResponse, error) {
//		return o.Client.Scenes.GetSceneList()
//	}); err != nil {
//		return fmt.Errorf("GetSceneList() failed: %w", err)
//	} else {
//		w.Send(pirog.ToJson(ret))
//	}
//	return nil
//}

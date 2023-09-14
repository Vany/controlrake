package widget

import (
	"bytes"
	"context"
	"fmt"
	"html/template"
	"io"
	"os"
	"path"
	"time"

	"github.com/ebitengine/oto/v3"
	"github.com/hajimehoshi/go-mp3"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/cont"
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

var _ = MustSurvive(RegisterWidgetType(&Button{}))

func (w *Button) Init(context.Context) error {
	err := mapstructure.Decode(w.Config.Args, &w.Args)
	return TERNARY(err == nil, nil, w.Errorf("cant read config %#v: %w", w.Config.Args, err))
}

func (w *Button) RenderTo(wr io.Writer) error {
	if err := TButton.Execute(wr, w); err != nil {
		return w.Errorf("render failed: %w", err)
	}
	return nil
}

var TButton = template.Must(template.New("Label").Parse(`
<div class="widget" id="{{.Name}}">
	<button onClick="Send(this, 'Boo')">⚙</button>
</div>
`))

func (w *Button) Consume(ctx context.Context, event []byte) error {
	con := cont.FromContext(ctx)
	con.Log.Log().Bytes("event", event).Msg("Pressed")

	if w.Args.Sound != "" {
		go func() {
			err := playSound(path.Join("sounds", w.Args.Sound))
			con.Log.Info().Err(err).Msg("Sound Played")
		}()
	}

	return nil
}

func playSound(fname string) error {
	fileBytes, err := os.ReadFile(fname)
	if err != nil {
		return fmt.Errorf("reading %s failed: %w", fname, err)
	}

	fileBytesReader := bytes.NewReader(fileBytes)
	decodedMp3, err := mp3.NewDecoder(fileBytesReader)
	if err != nil {
		return fmt.Errorf("decoding %s failed: %w", fname, err)
	}

	op := &oto.NewContextOptions{
		SampleRate:   44100,
		ChannelCount: 2,
		Format:       oto.FormatSignedInt16LE,
	}
	otoCtx, readyChan, err := oto.NewContext(op)
	if err != nil {
		return fmt.Errorf("NewContext() %s failed: %w", fname, err)

	}
	<-readyChan
	player := otoCtx.NewPlayer(decodedMp3)

	player.Play()

	for player.IsPlaying() {
		time.Sleep(time.Millisecond)
	}

	err = player.Close()
	if err != nil {
		return fmt.Errorf("player.Close() %s failed: %w", fname, err)

	}
	return nil
}

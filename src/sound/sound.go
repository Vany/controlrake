package sound

import (
	"bytes"
	"context"
	"fmt"
	"github.com/ebitengine/oto/v3"
	"github.com/hajimehoshi/go-mp3"
	"github.com/vany/controlrake/src/cont"
	"github.com/vany/controlrake/src/types"
	. "github.com/vany/pirog"
	"os"
	"path"
	"time"
)

// TODO🔴revisit me at some point

type SoundServer struct {
	BaseDir      string
	SoundContext *oto.Context
	FNameChan    chan string
	// TODO FileCache map[string][]byte
}

func (s *SoundServer) Play(ctx context.Context, fname string) error {
	s.FNameChan <- fname
	cont.FromContext(ctx).Log.Debug().Str("fname", fname).Send()
	return nil
}

func (s *SoundServer) PlayBlocked(fname string) error {
	fileBytes, err := os.ReadFile(path.Join(s.BaseDir, fname))
	if err != nil {
		return fmt.Errorf("reading %s failed: %w", fname, err)
	}

	fileBytesReader := bytes.NewReader(fileBytes)
	decodedMp3, err := mp3.NewDecoder(fileBytesReader)
	if err != nil {
		return fmt.Errorf("decoding %s failed: %w", fname, err)
	}

	player := s.SoundContext.NewPlayer(decodedMp3)

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

func New(ctx context.Context, basedir string) types.Sound {
	op := &oto.NewContextOptions{
		SampleRate:   44100,
		ChannelCount: 2,
		Format:       oto.FormatSignedInt16LE,
	}
	otoCtx, readyChan, err := oto.NewContext(op)
	if err != nil {
		MUST(fmt.Errorf("audio context creating failed: %w", err))

	}
	<-readyChan

	srv := &SoundServer{
		BaseDir:      basedir,
		SoundContext: otoCtx,
		FNameChan:    make(chan string, 10),
	}

	go func() {
		for {
			select {
			case fname := <-srv.FNameChan:
				cont.FromContext(ctx).Log.Debug().Str("fname", fname).Msg("Play sound")
				if err := srv.PlayBlocked(fname); err != nil {
					cont.FromContext(ctx).Log.Error().Err(err).Str("fname", fname).Msg("sound play failed")
				}
			case <-ctx.Done():
				cont.FromContext(ctx).Log.Debug().Msg("Sound routine shuted down")
				return
			}
		}
	}()

	return srv
}

package obs

import (
	"context"
	"fmt"
	"github.com/andreykaipov/goobs"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/cont"
	"github.com/vany/controlrake/src/types"
	. "github.com/vany/pirog"
	"time"
)

type Config struct {
	Server   string
	Password string
}

type Obs struct {
	Config
	Client *goobs.Client
	Cancel func()
}

// TODO provide logger to goobs
// TODO subscriptions in New()

func New(ctx context.Context) types.Obs {
	con := cont.FromContext(ctx)
	obs := &Obs{}
	mapstructure.Decode(con.Cfg.Obs, &obs.Config)

	obs.Init(ctx)
	//logger := con.Log.With().Str("component", "OBS").Logger()
	//obs.Client.Log = &logger

	return obs
}

func (o *Obs) Init(ctx context.Context) {
	cfctx, cf := context.WithCancel(ctx)
	o.Cancel = cf
	o.Client = nil

	go func() {
		pause := time.Microsecond
	OUT:
		for {
			select {
			case <-time.After(pause):
				if c, err := goobs.New(o.Config.Server, goobs.WithPassword(o.Config.Password)); err != nil {
					pause = TERNARY(pause > 10*time.Second, 10*time.Second, pause*2)
				} else {
					o.Client = c
					break OUT
				}
			}
		}

		<-cfctx.Done()
		err := o.Client.Disconnect()
		cont.FromContext(ctx).Log.Debug().Err(err).Msg("obs shut down")
	}()
}

func (o *Obs) Scenes(ctx context.Context) any {
	if sc, err := o.Client.Scenes.GetSceneList(); err != nil {
		return err
	} else {
		return sc
	}
}

func (o *Obs) Cli() *goobs.Client {
	return o.Client
}

// TODO  make channel reading with timeout here instead of .Client polling (MAKE IT FRAMEWORK to pirog)
// even not actual todo, rewrite this place if we will have more than one obs connection
var cantConnect = fmt.Errorf("cant connect to obs for five seconds")

func Wrapper[T any](ctx context.Context, o *Obs, f func() (T, error)) (T, error) {
	var zero T
	cnt := 5
	for o.Client == nil {
		<-time.After(time.Second)
		if cnt--; cnt < 0 {
			return zero, cantConnect
		}
	}
	if ret, err := f(); err != nil {
		o.Cancel()
		o.Init(ctx)
		return zero, err
	} else {
		return ret, nil
	}
}

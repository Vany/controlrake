package obs

import (
	"context"
	"github.com/andreykaipov/goobs"
	"github.com/andreykaipov/goobs/api/requests/record"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/cont"
	"github.com/vany/controlrake/src/types"
	"github.com/vany/pirog"
	"sync"
)

type Config struct {
	Server   string
	Password string
}

type Obs struct {
	Config
	Client *goobs.Client
	Mu     sync.Mutex // TODO remove it after library will be fixed
}

// TODO provide logger to goobs
// TODO subscriptions in New()

func New(ctx context.Context) types.Obs {
	con := cont.FromContext(ctx)
	cfg := Config{}
	mapstructure.Decode(con.Cfg.Obs, &cfg)
	obs := &Obs{
		Config: cfg,
		Client: pirog.MUST2(goobs.New(cfg.Server, goobs.WithPassword(cfg.Password))),
	}
	//logger := con.Log.With().Str("component", "OBS").Logger()
	//obs.Client.Log = &logger

	go func() {
		<-ctx.Done()
		obs.Transaction(func() {
			// todo why is it not return control flow
			err := obs.Client.Disconnect()
			cont.FromContext(ctx).Log.Debug().Err(err).Msg("obs shut down")
		})
	}()

	return obs
}

func (o *Obs) Scenes(ctx context.Context) any {
	if sc, err := o.Client.Scenes.GetSceneList(); err != nil {
		return err
	} else {
		return sc
	}
}

func (o *Obs) InfoRecord(ctx context.Context) *record.GetRecordStatusResponse {
	ret, err := o.Client.Record.GetRecordStatus()
	if err != nil {
		cont.FromContext(ctx).Log.Error().Err(err).Msg("obs GetRecordStatus failed")
	}
	return ret
}

func (o *Obs) Cli() *goobs.Client {
	return o.Client
}

func (o *Obs) Transaction(f func()) {
	o.Mu.Lock()
	defer o.Mu.Unlock()
	f()
}

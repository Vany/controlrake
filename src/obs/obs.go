package obs

import (
	"context"
	"github.com/andreykaipov/goobs"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	obs_api "github.com/vany/controlrake/src/obs/api"
	. "github.com/vany/pirog"
	"sync"
	"time"
)

// TODO:
///	📍cycled connection routine, it need to check tht here is no connection and try to connect.
/// 📍request via SUBSCRIPTION(REQUEST(req, resp))

// Obs - perform requests to obs.
type Obs struct {
	Config *config.ConfigComponent `inject:"Config"`
	Logger *zerolog.Logger         `inject:"Logger"`

	Cfg    *obs_api.Config
	Client *goobs.Client
	CliMu  sync.Mutex
}

func New() *Obs {
	return &Obs{}
}

func (o *Obs) Init(ctx context.Context) error {
	o.Cfg = &o.Config.Obs
	o.Logger = REF(o.Logger.With().Str("comp", "obs").Logger())
	o.Logger.Info().Msg("initializing")
	o.connect(ctx)
	o.Logger.Info().Msg("Connected")
	return nil
}

func (o *Obs) Run(ctx context.Context) error { return nil }

func (o *Obs) Stop(ctx context.Context) error {
	o.disconnect()
	return nil
}

func (o *Obs) Cli() *goobs.Client {
	return o.Client
}

// todo - use pirog.request here.
// func Wrapper[T any](o *Obs, f func() (T, error)) (T, error) {
//var zero T
//if ret, err := f(); err != nil {
//	o.connect()
//	return zero, err
//} else {
//	return ret, nil
//}
// }

func (o *Obs) connect(ctx context.Context) (err error) {
	o.CliMu.Lock()
	defer o.CliMu.Unlock()
	o.disconnect()
	pause := time.Microsecond
	for {
		<-time.After(pause)
		if ctx.Err() != nil {
			return ctx.Err()
		}
		if cli, err := goobs.New(o.Cfg.Server, goobs.WithPassword(o.Cfg.Password)); err == nil {
			o.Client = cli
			break

		} else if err := ctx.Err(); err != nil {
			return err
		}
		o.Logger.Error().Err(err).Msg("Connecting")

		pause = TERNARY(pause > 10*time.Second, 10*time.Second, pause*2)
	}

	o.Client, err = goobs.New(o.Cfg.Server, goobs.WithPassword(o.Cfg.Password))
	return err
}

func (o *Obs) disconnect() {
	if o.Client != nil {
		o.Client.Disconnect()
		o.Client = nil
	}
}

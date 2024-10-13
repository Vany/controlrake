package obs

import (
	"context"
	"github.com/andreykaipov/goobs"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	obs_api "github.com/vany/controlrake/src/obs/api"
	. "github.com/vany/pirog"
	"time"
)

// Obs - component holds connector to obs.
type Obs struct {
	Cfg    *obs_api.Config
	Config *config.Config  `inject:"Config"`
	Logger *zerolog.Logger `inject:"Logger"`
	Client *goobs.Client
}

func New() *Obs { return &Obs{} }

func (o *Obs) Init(ctx context.Context) error {
	return nil
	o.Cfg = &o.Config.Obs
	o.Logger = REF(o.Logger.With().Str("comp", "obs").Logger())
	pause := time.Microsecond
	for {
		<-time.After(pause)
		var err error
		if o.Client, err = goobs.New(o.Cfg.Server, goobs.WithPassword(o.Cfg.Password)); err == nil {
			break
		}
		pause = TERNARY(pause > 10*time.Second, 10*time.Second, pause*2)
	}

	o.Logger.Info().Msg("Connected")
	return nil
}

func (o *Obs) Run(ctx context.Context) error { return nil }

func (o *Obs) Stop(ctx context.Context) error {
	return o.Client.Disconnect()
}

func (o *Obs) Cli() *goobs.Client {
	return o.Client
}

// todo - use pirog.request here.

func Wrapper[T any](o *Obs, f func() (T, error)) (T, error) {
	var zero T
	if ret, err := f(); err != nil {
		o.reconnect()
		return zero, err
	} else {
		return ret, nil
	}
}

func (o *Obs) reconnect() (err error) {
	if o.Client != nil {
		o.Client.Disconnect()
	}
	o.Client, err = goobs.New(o.Cfg.Server, goobs.WithPassword(o.Cfg.Password))
	return err
}

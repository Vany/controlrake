package obs

import (
	"context"
	"errors"
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

	Cfg        *obs_api.Config
	Client     *goobs.Client
	ClientMu   sync.Mutex
	ClientDead chan struct{}
}

func New() *Obs {
	return &Obs{}
}

func (o *Obs) Init(ctx context.Context) error {
	o.Cfg = &o.Config.Obs
	o.Logger = REF(o.Logger.With().Str("comp", "obs").Logger())

	go func() {
		for {
			if o.Client == nil {
				o.connect(ctx)
			}
			select {
			case <-ctx.Done():
				return
			case <-o.ClientDead:
				o.connect(ctx)
			}
		}
	}()

	return nil
}

func (o *Obs) Run(ctx context.Context) error { return nil }

func (o *Obs) Stop(ctx context.Context) error {
	o.disconnect()
	return nil
}

var NotConnected = errors.New("obs not connected")

// Execute
func (o *Obs) Execute(f func(*goobs.Client) error) error {
	o.ClientMu.Lock()
	defer o.ClientMu.Unlock()
	for range 5 {
		if o.Client != nil {
			break
		}
		<-time.After(time.Second)
	}
	if o.Client == nil {
		o.ClientDead <- struct{}{}
		return NotConnected
	}

	err := f(o.Client)
	if err != nil {
		o.ClientDead <- struct{}{}
	}
	return err
}

func (o *Obs) Cli() *goobs.Client {
	return o.Client
}

func (o *Obs) connect(ctx context.Context) (err error) {
	o.Client = nil
	o.ClientMu.Lock()
	defer o.ClientMu.Unlock()
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
		o.ClientMu.Lock()
		defer o.ClientMu.Unlock()
		o.Client.Disconnect()
		o.Client = nil
	}
}

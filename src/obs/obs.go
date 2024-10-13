package obs

import (
	"context"
	"errors"
	"github.com/andreykaipov/goobs"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/obs/api"
	. "github.com/vany/pirog"
	"reflect"
	"sync/atomic"
	"time"
)

// Obs - obs connection service routine.
type Obs struct {
	Config *config.ConfigComponent `inject:"Config"`
	Logger *zerolog.Logger         `inject:"Logger"`

	Cfg                      *api.Config
	Client                   atomic.Pointer[goobs.Client]
	connecting               atomic.Bool
	availabilytyNotification *SUBSCRIPTION[bool, struct{}]    // notify that obs is ready to operate
	eventStream              *SUBSCRIPTION[reflect.Type, any] // Obs events
}

func New() *Obs {
	return &Obs{
		availabilytyNotification: NewSUBSCRIPTION[bool, struct{}](),
		eventStream:              NewSUBSCRIPTION[reflect.Type, any](),
	}
}

func (o *Obs) Init(ctx context.Context) error {
	o.Cfg = &o.Config.Obs
	o.Logger = REF(o.Logger.With().Str("comp", "obs").Logger())

	go func() {
		dead := o.availabilytyNotification.Subscribe(false)
		for {
			if o.Client.Load() == nil {
				go o.connect(ctx)
			}
			select {
			case <-ctx.Done():
				return
			case <-dead:
				continue
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

// connect - try to connect until we are connected or need exit
func (o *Obs) connect(ctx context.Context) {
	if !o.connecting.CompareAndSwap(false, true) {
		o.Logger.Log().Msg("already connecting")
		return
	}
	defer o.connecting.Store(false)
	o.Logger.Log().Msg("start connecting")
	o.Client.Store(nil)

	for {
		if cli, err := goobs.New(o.Cfg.Server, goobs.WithPassword(o.Cfg.Password)); err == nil {
			o.Client.Store(cli)
			break
		} else if ctx.Err() != nil {
			return
		} else {
			o.Logger.Debug().Err(err).Msg("connection failed")
		}
		<-time.After(time.Second)
	}

	go func() {
		o.Logger.Log().Msg("event stream tapped")
		o.availabilytyNotification.Notify(true, struct{}{})
		for e := range o.Client.Load().IncomingEvents {
			o.eventStream.Notify(api.AllEvents, e)
			if t := reflect.TypeOf(e); o.eventStream.Has(t) {
				o.eventStream.Notify(t, e)
			}
		}
		o.Logger.Log().Msg("event stream closed")
		o.Client.Store(nil)
		o.availabilytyNotification.Notify(false, struct{}{})
	}()
	return
}

func (o *Obs) disconnect() {
	if o.Cli() != nil {
		o.Client.Load().Disconnect()
		o.Client.Store(nil)
	}
}

func (o *Obs) Cli() *goobs.Client                            { return o.Client.Load() }
func (o *Obs) EventStream() *SUBSCRIPTION[reflect.Type, any] { return o.eventStream }
func (o *Obs) AvailabilityNotification() *SUBSCRIPTION[bool, struct{}] {
	return o.availabilytyNotification
}

var NotConnected = errors.New("obs not connected")

// Execute - in obs context
func (o *Obs) Execute(f func(*goobs.Client) error) error {
	for range 5 {
		if o.Cli() != nil {
			err := f(o.Cli())
			if err != nil {
				o.Client.Store(nil)
			}
			return err
		}
		<-time.After(time.Second)
	}
	return NotConnected
}

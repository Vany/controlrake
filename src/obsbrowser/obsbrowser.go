// package obsbrowser: code for connecting to html page in obs

package obsbrowser

import (
	"context"
	"fmt"
	"github.com/google/uuid"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/obsbrowser/api"
	"github.com/vany/pirog"
	"strings"
	"sync"
	"time"
)

type ObsBrowser struct {
	Cfg    *api.Config
	Config *config.ConfigComponent `inject:"Config"`
	Logger *zerolog.Logger         `inject:"Logger"`

	ToWebChan     chan string
	Receivers     map[uuid.UUID]*api.Request
	LastAccessed  map[uuid.UUID]time.Time
	Mu            sync.Mutex
	ToWebRequests chan *api.Request
}

func (o *ObsBrowser) WebSpittoon() chan string { return o.ToWebChan }

func New() *ObsBrowser {
	return &ObsBrowser{
		ToWebChan:     make(chan string, 1),
		ToWebRequests: make(chan *api.Request),
		Receivers:     make(map[uuid.UUID]*api.Request),
		LastAccessed:  make(map[uuid.UUID]time.Time),
	}
}

func (o *ObsBrowser) Init(ctx context.Context) error {
	o.Cfg = &o.Config.ObsBrowser
	o.Logger = pirog.REF(o.Logger.With().Str("comp", "obsws").Logger())

	go func() { // todo refactor this to something like request/response.
		select {
		case <-ctx.Done():
			return

		case now := <-time.Tick(30 * time.Second):
			o.Logger.Debug().Msg("obs browser garbage collection")
			t := now.Add(-10 * time.Minute)
			o.Mu.Lock()
			for k, v := range o.LastAccessed {
				if v.Before(t) {
					o.CloseObject(ctx, k)
				}
			}
			o.Mu.Unlock()
		}
	}()

	return nil
}

func (o *ObsBrowser) Run(ctx context.Context) error  { return nil }
func (o *ObsBrowser) Stop(ctx context.Context) error { return nil }

// ToWeb - Channel to send requests to webpage in obs browser
func (o *ObsBrowser) ToWeb(ctx context.Context, msg string) *api.Request {
	r := pirog.REQUEST[string, string](msg)
	uuid := uuid.New()
	pirog.SEND(ctx, o.ToWebChan, uuid.String()+"|"+msg)
	o.Mu.Lock()
	o.Receivers[uuid] = &r
	o.LastAccessed[uuid] = time.Now()
	o.Mu.Unlock()
	o.Logger.Debug().Str("msg", r.REQ).Msg("Sent")
	return &r
}

// WebIngest - ws handler for httpserver
func (o *ObsBrowser) WebIngest(ctx context.Context, data string) error {
	parts := strings.SplitN(data, "|", 2)
	if uuid, err := uuid.Parse(parts[0]); err != nil {
		return fmt.Errorf("parse uuid %s: %w", parts[0], err)
	} else if r, ok := o.Receivers[uuid]; !ok {
		return fmt.Errorf("SendObject(%s) not found", parts[0])
	} else if strings.HasPrefix(parts[1], "close") {
		o.Mu.Lock()
		o.CloseObject(ctx, uuid)
		o.Mu.Unlock()
	} else {
		r.RESPOND(ctx, parts[1])
		o.Mu.Lock()
		o.LastAccessed[uuid] = time.Now()
		o.Mu.Unlock()
	}
	return nil
}

func (o *ObsBrowser) CloseObject(ctx context.Context, u uuid.UUID) {
	o.Logger.Debug().Str("uuid", u.String()).Msg("removed")
	if r, ok := o.Receivers[u]; ok {
		close(r.RES)
	}
	delete(o.Receivers, u)
	delete(o.LastAccessed, u)
}

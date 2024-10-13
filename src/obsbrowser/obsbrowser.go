// package obsbrowser: code for connecting to html page in obs

package obsbrowser

import (
	"context"
	"fmt"
	"github.com/google/uuid"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/obsbrowser/api"
	"github.com/vany/pirog"
	"strings"
	"sync"
	"time"
)

type Browser struct {
	Logger *zerolog.Logger `inject:"Logger"`

	Chan         chan string
	Receivers    map[uuid.UUID]*SendObject
	LastAccessed map[uuid.UUID]time.Time
	Mu           sync.Mutex
}

func New() *Browser {
	return &Browser{
		Chan:         make(chan string, 1),
		Receivers:    make(map[uuid.UUID]*SendObject),
		LastAccessed: make(map[uuid.UUID]time.Time),
	}
}

func (o *Browser) Init(ctx context.Context) error {
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

func (o *Browser) Run(ctx context.Context) error  { return nil }
func (o *Browser) Stop(ctx context.Context) error { return nil }

func (o *Browser) SendChan() chan string { return o.Chan }

// TODO  optimize cleaner with priority queue
func (o *Browser) Send(ctx context.Context, msg string) api.ObsSendObject {
	uuid := uuid.New()
	o.Chan <- uuid.String() + "|" + msg
	ret := &SendObject{
		DoneChan:    make(chan struct{}),
		ReceiveChan: make(chan string),
	}
	o.Mu.Lock()
	o.Receivers[uuid] = ret
	o.LastAccessed[uuid] = time.Now()
	o.Mu.Unlock()

	o.Logger.Debug().Str("msg", msg).Msg("Sent")
	return ret
}

func (o *Browser) Dispatch(ctx context.Context, b string) error {
	parts := strings.SplitN(b, "|", 2)
	if uuid, err := uuid.Parse(parts[0]); err != nil {
		return fmt.Errorf("parse uuid %s: %w", parts[0], err)
	} else if so, ok := o.Receivers[uuid]; !ok {
		return fmt.Errorf("SendObject(%s) not found", parts[0])
	} else if strings.HasPrefix(parts[1], "done") {
		o.Mu.Lock()
		o.CloseObject(ctx, uuid)
		o.Mu.Unlock()
	} else {
		so.ReceiveChan <- parts[1]
		o.Mu.Lock()
		o.LastAccessed[uuid] = time.Now()
		o.Mu.Unlock()
	}
	return nil
}

func (o *Browser) CloseObject(ctx context.Context, u uuid.UUID) {
	o.Logger.Debug().Str("uuid", u.String()).Msg("removed")
	so, ok := o.Receivers[u]
	if ok {
		close(so.DoneChan)
	}
	delete(o.Receivers, u)
	delete(o.LastAccessed, u)
}

type SendObject struct {
	DoneChan    chan struct{}
	ReceiveChan chan string
}

func (s *SendObject) Done() chan struct{} {
	return s.DoneChan
}

func (s *SendObject) Receive() chan string {
	return s.ReceiveChan
}

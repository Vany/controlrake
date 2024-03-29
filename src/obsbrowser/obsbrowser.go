// package obsbrowser: code for connecting to html page in obs

package obsbrowser

import (
	"context"
	"fmt"
	"github.com/google/uuid"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/types"
	"strings"
	"sync"
	"time"
)

type Browser struct {
	Chan         chan string
	Receivers    map[uuid.UUID]*SendObject
	LastAccessed map[uuid.UUID]time.Time
	Mu           sync.Mutex
}

func New(ctx context.Context) *Browser {
	self := Browser{
		Chan:         make(chan string, 1),
		Receivers:    make(map[uuid.UUID]*SendObject),
		LastAccessed: make(map[uuid.UUID]time.Time),
	}

	go func() {
		select {
		case <-ctx.Done():
			return
		case now := <-time.Tick(30 * time.Second):
			app.FromContext(ctx).Log.Debug().Msg("obs browser garbage collection")
			t := now.Add(-10 * time.Minute)
			self.Mu.Lock()
			defer self.Mu.Unlock()
			for k, v := range self.LastAccessed {
				if v.Before(t) {
					self.CloseObject(ctx, k)
				}
			}
		}
	}()
	return &self
}

func (o *Browser) SendChan() chan string { return o.Chan }

// TODO  optimize cleaner with priority queue
func (o *Browser) Send(ctx context.Context, msg string) types.ObsSendObject {
	log := app.FromContext(ctx).Logger()
	uuid := uuid.New()
	o.Chan <- uuid.String() + "|" + msg
	ret := &SendObject{
		DoneChan:    make(chan struct{}),
		ReceiveChan: make(chan string),
	}
	o.Mu.Lock()
	defer o.Mu.Unlock()
	o.Receivers[uuid] = ret
	o.LastAccessed[uuid] = time.Now()

	log.Debug().Str("msg", msg).Msg("Sent")
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
		defer o.Mu.Unlock()
		o.CloseObject(ctx, uuid)
	} else {
		so.ReceiveChan <- string(parts[1])
		o.Mu.Lock()
		defer o.Mu.Unlock()
		o.LastAccessed[uuid] = time.Now()
	}
	return nil
}

func (o *Browser) CloseObject(ctx context.Context, u uuid.UUID) {
	app.FromContext(ctx).Log.Debug().Str("uuid", u.String()).Msg("removed")
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

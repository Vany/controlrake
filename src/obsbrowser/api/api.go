package api

import "context"

type ObsBrowser interface {
	Send(ctx context.Context, msg string) ObsSendObject // send message to obs browser html
	Dispatch(ctx context.Context, b string) error       // receive event from obs browser html
	SendChan() chan string                              // channel from server to page
}

type ObsSendObject interface {
	Done() chan struct{}  // will be closed when action was finished
	Receive() chan string // return action progress messages channel
}

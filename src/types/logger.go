package types

import (
	"context"
	"github.com/rs/zerolog"
	"os"
)

type Logger struct{ zerolog.Logger }

func CreateLoggerToContext(ctx context.Context) context.Context {
	log := Logger{zerolog.New(os.Stdout)}
	return PutToContext(ctx, &log)
}

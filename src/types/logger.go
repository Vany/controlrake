package types

import (
	"github.com/rs/zerolog"
	"os"
)

type Logger struct{ zerolog.Logger }

func NewLogger() *Logger {
	return &Logger{zerolog.New(os.Stdout)}
}

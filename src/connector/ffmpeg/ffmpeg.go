package ffmpeg

import (
	"github.com/Vany/controlrake/src/types"
)

type FFMpeg struct {
}

func (o *FFMpeg) Init() types.Connector {
	return o
}

func (o *FFMpeg) Handle(method string, arg interface{}) {
	panic("implement me")
}

func New() *FFMpeg {
	return new(FFMpeg)
}

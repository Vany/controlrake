// Widget is inner representation of functionality, taht connects web and server part.
package widget

import (
	"context"
	"fmt"
	. "github.com/vany/pirog"
	"io"
)

type Registry map[string]Widget

func NewRegistry(ctx context.Context, confs []Config) Registry {
	r := make(Registry)
	for _, v := range confs {
		r[v.Name] = New(ctx, v)
	}
	return r
}

type Widget interface {
	Init() error                     // init widget with config in it's base
	RenderTo(writer io.Writer) error // render visual representation
}

type Config struct {
	Name string
	Type string
	Args any
}

func New(ctx context.Context, cfg Config) Widget {
	var w Widget
	b := BaseWidget{cfg}
	switch cfg.Type {
	case "Label":
		w = &Label{BaseWidget: b}
	default:
		panic("unknown widget type: " + cfg.Type)
	}
	MUST(w.Init())
	return w
}

type BaseWidget struct {
	Config
}

func (w BaseWidget) RenderTo(wr io.Writer) error {
	_, err := wr.Write([]byte(w.Name + " => " + w.Type))
	return err
}

func (w BaseWidget) Init() error {
	panic("unimplemented")
}

func (w BaseWidget) Errorf(f string, args ...any) error {
	return fmt.Errorf("name: %s, type: %s "+f, w.Name, w.Type, args)
}

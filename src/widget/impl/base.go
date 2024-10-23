// package widget: Widget is inner representation of functionality, that connects web and server part.
package impl

import (
	"context"
	"fmt"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/widget/api"
	"html/template"
	"io"
	"reflect"
)

type RawHTML string

func (r RawHTML) String() string { return string(r) } // template.JS

var TypeRegistry = make(map[string]reflect.Type)     // typename => widgettype
var TemplateStringRegistry = make(map[string]string) // typename => html.template
var TemplateRegistry = make(map[string]*template.Template)

func RegisterWidgetType(w api.Widget, tmplstring string) struct{} {
	t := reflect.TypeOf(w).Elem()
	TypeRegistry[t.Name()] = t
	TemplateStringRegistry[t.Name()] = tmplstring
	return struct{}{}
}

type BaseWidget struct {
	api.WidgetConfig
	Widget api.Widget     // link to actual widget object
	Log    zerolog.Logger // logger for specified widget
}

func (w *BaseWidget) Init(context.Context, api.WidgetConstructor) error { return nil }
func (w *BaseWidget) Base() *BaseWidget                                 { return w }
func (w *BaseWidget) TemplateWrap(str string) string {
	return fmt.Sprintf(`<div class="widget" id="{{.Name}}" {{if .Style}}style="{{.Style}}"{{end}}>%s</div>`, str)
}

// Consume websocket message in separate goroutine
func (w *BaseWidget) Dispatch(ctx context.Context, event string) error {
	return w.Errorf("Dispatch() is Virtual")
}

func (w *BaseWidget) RenderTo(ctx context.Context, arg string, wr io.Writer) error {
	if _, ok := TemplateRegistry[w.Type]; !ok {
		if str, ok := TemplateStringRegistry[w.Type]; !ok {
			return w.Errorf("template not found")
		} else if x, err := template.New(w.Type).Funcs(w.Widget.RenderFuncs()).Parse(w.TemplateWrap(str)); err != nil {
			return w.Errorf("template parse error: %w", err)
		} else {
			TemplateRegistry[w.Type] = x
		}
	}
	return TemplateRegistry[w.Type].Execute(wr, w.Widget)
}

func (w *BaseWidget) Errorf(f string, args ...any) error {
	return fmt.Errorf("%s(%s)"+f, append([]any{w.Type, w.Name}, args...)...)
}

func (w *BaseWidget) RenderFuncs() map[string]any {
	return make(map[string]any)
}

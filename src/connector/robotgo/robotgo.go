package robotgo

//go:generate perl ../../../generate.pl

import (
	"github.com/Vany/controlrake/src/types"
	"github.com/go-vgo/robotgo"
	"log"
	"strings"
)

/*PERLCODE
$STASH{methods} = [qw(write)];
*/

type RobotGo struct {
}

func (o *RobotGo) Init() types.Connector {
	return o
}

func (o *RobotGo) Handle(method string, arg interface{}) {
	log.Printf("ROBO: %#v", arg)

	args, ok := arg.([]interface{})
	if !ok {
		log.Printf("arg is not an array")
		return
	}
	switch method {
	//BEGIN dispatch
	case "write":
		o.Method_write(args...)

		//END dispatch
	}
}

func New() *RobotGo {
	return new(RobotGo)
}

func (o *RobotGo) Method_write(strs ...interface{}) {
	for _, s := range strs {
		for _, k := range strings.Split(s.(string), "\n") {
			robotgo.TypeString(k)
			robotgo.KeyTap("enter")
		}
	}
}

/*PERLCODE

my $buff = "";
for my $m (@{ $STASH{methods} }) {
	$buff .= "\tcase \"$m\": \n".
		"\t\to.Method_$m(args...)\n";
}
putblock dispatch => $buff;

*/

Highcharts.theme = {
	colors: ['#ac4142', '#90a959', '#f4bf75', '#6a9fb5', '#aa759f', '#75b5aa', '#752a2a', '#5d742a', '#754e2a', '#2a4e74', '#703664', '#297366'],
	chart: {
  	backgroundColor: "#222"
  },
  title: {
  	style: {
    	color: '#ccc'
    }
  },
  legend: {
  	itemStyle: {
    	color: '#ccc'
    },
    itemHoverStyle: {
    	color: '#fff'
    }
  },
  xAxis: {
  	labels: {
    	style: {
      	color: '#ccc'
      }
    }
  },
  yAxis: {
  	title: {
    	style: {
      	color: '#ccc'
      }
    },
  	labels: {
    	style: {
      	color: '#ccc'
      }
    }
  }
};
Highcharts.setOptions(Highcharts.theme);

function parse_series(branch) {
  var series_name = branch.name;
  var series_data = [];
  for (var i = 0; i < branch.results.length; i++) {
    var result = branch.results[i];
    series_data.push([Date.parse(result.timestamp), result.avg]);
  }
  return { data: series_data, name: series_name };
}

function add_chart(bench) {
  var chart_name = bench.name;
  var chart_series = [];
  for (var i = 0; i < bench.branches.length; i++) {
    chart_series.push(parse_series(bench.branches[i]));
  }

  // Calculate chart range
  var min = Number.MAX_SAFE_INTEGER;
  var max = 0;
  for (var i = 0; i < chart_series.length; i++) {
    for (var j = 0; j < chart_series[i].data.length; j++) {
      var avg = chart_series[i].data[j][1];
      if (avg > max) {
        max = avg;
      }
      if (avg < min) {
        min = avg;
      }
    }
  }
  var chart_avg = (min + max) / 2;
  var chart_min = Math.min(min, avg * 0.90);
  var chart_max = Math.max(max, avg * 1.10);

  Highcharts.chart(chart_name, {
    title: {
      text: chart_name,
    },
    yAxis: {
      title: {
        text: 'Seconds'

      },
      min: chart_min,
      max: chart_max,
    },
    xAxis: {
      type: 'datetime'
    },
    series: chart_series
  });
}


function dataResponseCallback(body) {
  var data_json = JSON.parse(body);
  for (var i = 0; i < data_json.length; i++) {
    var chart = add_chart(data_json[i]);
  }
}

var xmlHttp = new XMLHttpRequest();
xmlHttp.onreadystatechange = function() {
    if (xmlHttp.readyState == 4 && xmlHttp.status == 200) {
        dataResponseCallback(xmlHttp.responseText);
    }
}
xmlHttp.open("GET", "/data", true);
xmlHttp.send(null);

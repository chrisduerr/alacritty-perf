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

  Highcharts.chart(chart_name, {
    title: {
      text: chart_name,
    },
    yAxis: {
      title: {
        text: 'Seconds'
      }
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

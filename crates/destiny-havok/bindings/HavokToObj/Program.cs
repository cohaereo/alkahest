using System.Text;
using HavokToObj;

var buffer = File.ReadAllBytes(args[0]);
var shapeCollection = DestinyHavok.ReadShapeCollection(buffer);

Directory.CreateDirectory("shapes");
int i = 0;
foreach (var shape in shapeCollection)
{
    var vertices = shape.Vertices;
    var indices = shape.Indices;
    
    var sb = new StringBuilder();
    foreach (var vertex in vertices)
    {
        sb.AppendLine($"v {vertex.X} {vertex.Y} {vertex.Z}");
    }
    foreach (var index in indices.Chunk(3))
    {
        sb.AppendLine($"f {index[0] + 1} {index[1] + 1} {index[2] + 1}");
    }
    
    Console.WriteLine($"Writing 'shapes/shape_{i}.obj'");
    File.WriteAllText($"shapes/shape_{i++}.obj", sb.ToString());
}